use std::{
    env,
    f32::consts::E,
    mem::take,
    sync::Arc,
    time::{Duration, SystemTime},
};

use poulpe_ethercat_controller::PoulpeController;
use tokio::{sync::mpsc, time::{error::Elapsed, sleep}};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{transport::Server, Request, Response, Status, Streaming};

use poulpe_ethercat_grpc::pb::{
    poulpe_multiplexer_server::{PoulpeMultiplexer, PoulpeMultiplexerServer},
    PoulpeCommands, PoulpeIds, PoulpeState, PoulpeStates, StateStreamRequest
};

use prost_types::Timestamp;
#[derive(Debug)]
struct PoulpeMultiplexerService {
    controller: Arc<PoulpeController>,
}

fn get_state_for_id(controller: &PoulpeController, id: i32) -> Result<PoulpeState, Box<dyn std::error::Error>> {
    let slave_id = id as u32;

    if controller.get_slave_ids().contains(&slave_id) == false{
        log::error!("Invalid slave id {}", slave_id);
        return Err(("Invalid slave id").into());
    }

    Ok(PoulpeState {
        id,
        compliant: match controller.is_torque_on(slave_id) {
            Ok(Some(state)) => state,
            _ => {
                log::error!("Failed to get compliant state for slave {}", slave_id);
                false
            }
        },
        actual_position: match controller.get_current_position(slave_id) {
            Ok(Some(pos)) => pos,
            _ => {
                log::error!("Failed to get actual position for slave {}", slave_id);
                vec![0.0; controller.get_orbita_type(slave_id) as usize]
            }
        },
        actual_velocity: match controller.get_current_velocity(slave_id) {
            Ok(Some(vel)) => vel,
            _ => {
                log::error!("Failed to get actual velocity for slave {}", slave_id);
                vec![0.0; controller.get_orbita_type(slave_id) as usize]
            }
        },
        actual_torque: match controller.get_current_torque(slave_id) {
            Ok(Some(torque)) => torque,
            _ => {
                log::error!("Failed to get actual torque for slave {}", slave_id);
                vec![0.0; controller.get_orbita_type(slave_id) as usize]
            }
        },
        axis_sensors: match controller.get_current_axis_sensors(slave_id) {
            Ok(Some(sensor)) => sensor,
            _ => {
                log::error!("Failed to get axis sensor for slave {}", slave_id);
                vec![0.0; controller.get_orbita_type(slave_id) as usize]
            }
        },
        requested_target_position: match controller.get_current_target_position(slave_id) {
            Ok(Some(pos)) => pos,
            _ => {
                log::error!(
                    "Failed to get requested target position for slave {}",
                    slave_id
                );
                vec![0.0; controller.get_orbita_type(slave_id) as usize]
            }
        },
        state: controller.get_status(slave_id) as u32,
        torque_state: match controller.is_torque_on(slave_id) {
            Ok(Some(state)) => state,
            _ => {
                log::error!("Failed to get torque state for slave {}", slave_id);
                false
            }
        },
    })
}

#[tonic::async_trait]
impl PoulpeMultiplexer for PoulpeMultiplexerService {
    async fn get_poulpe_ids(&self, _request: Request<()>) -> Result<Response<PoulpeIds>, Status> {
        let reply = PoulpeIds {
            ids: self
                .controller
                .get_slave_ids()
                .iter()
                .map(|&id| id as i32)
                .collect(),
            names: self
                .controller
                .get_slave_names()
        };

        Ok(Response::new(reply))
    }

    type GetStatesStream = ReceiverStream<Result<PoulpeStates, Status>>;

    async fn get_states(
        &self,
        request: Request<StateStreamRequest>,
    ) -> Result<Response<Self::GetStatesStream>, Status> {
        let (tx, rx) = mpsc::channel(4);

        let controller = self.controller.clone();

        tokio::spawn(async move {
            let request = request.get_ref();

            loop {
                
                let states = PoulpeStates {
                    states: request
                        .ids
                        .iter()
                        .map(|&id| 
                            match get_state_for_id(&controller, id){
                                Ok(state) => state,
                                Err(e) => {
                                    log::error!("Failed to get state for slave {}: {}", id, e);
                                    PoulpeState {
                                        id,
                                        compliant: false,
                                        actual_position: vec![],
                                        actual_velocity: vec![],
                                        actual_torque: vec![],
                                        axis_sensors: vec![],
                                        requested_target_position: vec![],
                                        state: 0,
                                        torque_state: false,
                                    }
                                }
                            }

                        )
                        .collect(),
                };

                if tx.send(Ok(states)).await.is_err() {
                    break;
                }

                sleep(Duration::from_secs_f32(request.update_period)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_commands(
        &self,
        request: Request<Streaming<PoulpeCommands>>,
    ) -> Result<Response<()>, Status> {
        let mut stream = request.into_inner();

        let mut t = SystemTime::now();
        let mut nb = 0;
        let mut command_times:u128 = 0;

        let mut elapsed_time = 0;
        while let Some(Ok(req)) = stream.next().await {
            log::debug!("Got commands {:?}", req);
            for cmd in req.commands {
                let slave_id = cmd.id as u32;

                match cmd.published_timestamp{
                    Some(published_time) => {
                        let published_time = match SystemTime::try_from(published_time) {
                            Ok(systime) => systime,
                            Err(_) => {
                                log::warn!("Cannot parse the timestamp, discarding message!");
                                continue;
                            }
                        };
                        // check if the message is older than 10 ms
                        elapsed_time = published_time.elapsed().unwrap().as_millis();
                        if elapsed_time > 5 {
                            log::warn!("Message older than {} ms, discarding!", 5);
                            continue;
                        }
                    }
                    None => {
                        log::warn!("No published timestamp, discarding message!");
                        continue;
                    }
                }

                let no_axis = self.controller.get_orbita_type(slave_id) as usize;

                let mut set_compliant = cmd.compliancy;

                let target_pos = cmd.target_position;
                if target_pos.len() != 0 {
                    // set only last target command 
                    self.controller.set_target_position(
                        slave_id, 
                        target_pos[(target_pos.len()-no_axis)..].to_vec()
                    ).unwrap_or_else(|e| {
                        log::error!("Failed to set target position for slave {}: {}", slave_id, e);
                        set_compliant = Some(true); // disable the slave!
                    });
                }
                let velocity_limit = cmd.velocity_limit;
                if velocity_limit.len() != 0 {
                    // set only last target command 
                    self.controller.set_velocity_limit(
                        slave_id, 
                        velocity_limit[(velocity_limit.len()-no_axis)..].to_vec()
                    ).unwrap_or_else(|e| {
                        log::error!("Failed to set velocity limit for slave {}: {}", slave_id, e);
                        set_compliant = Some(true); // disable the slave!
                    });
                }
                let torque_limit = cmd.torque_limit;
                if torque_limit.len() != 0 {
                    // set only last target command 
                    self.controller.set_torque_limit(
                        slave_id, 
                        torque_limit[(torque_limit.len()-no_axis)..].to_vec()
                    ).unwrap_or_else(|e| {
                        log::error!("Failed to set torque limit for slave {}: {}", slave_id, e);
                        set_compliant = Some(true); // disable the slave!
                    });
                }

                // check if the slave is compliant
                match set_compliant { 
                    Some(true) => self.controller.set_torque(slave_id, false).unwrap_or_else(
                        |e| log::error!("Failed to set torque off for slave {}: {}", slave_id, e),
                    ),
                    Some(false) => self.controller.set_torque(slave_id, true).unwrap_or_else(
                        |e| log::error!("Failed to set torque on for slave {}: {}", slave_id, e),
                    ),
                    None => (),
                }

            }

            nb += 1;
            command_times += elapsed_time;
            let dt = t.elapsed().unwrap().as_secs_f32();
            if dt > 1.0 {
                let f = nb as f32 / dt;
                let dt_c = (command_times as f32) / (nb as f32);
                log::info!("Got {} req/s, average execution time: {} ms", f, dt_c);

                t = SystemTime::now();
                command_times = 0;
                nb = 0;
            }
        }

        Ok(Response::new(()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<_> = env::args().collect();
    let filename = match args.len() {
        2 => &args[1],
        _ => {
            println!("usage: {} ESI-FILE", env!("CARGO_PKG_NAME"));
            return Ok(());
        }
    };

    let controller = PoulpeController::connect(filename)?;

    for slave_id in controller.get_slave_ids() {
        log::info!("Setup Slave {}...", slave_id);
        match controller.setup(slave_id) {
            Ok(_) => log::info!("Done!"),
            Err(e) => {
                log::error!("Failed to setup slave {}: {}", slave_id, e);
                Err(e)?;
            }
        }
    }

    log::info!("POULPE controller ready!");

    let addr = "[::]:50098".parse()?;
    let srv = PoulpeMultiplexerService {
        controller: Arc::new(controller),
    };

    Server::builder()
        .add_service(PoulpeMultiplexerServer::new(srv))
        .serve(addr)
        .await?;

    Ok(())
}
