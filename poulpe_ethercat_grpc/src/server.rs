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
                return Err("Failed to get compliant state".into());
            }
        },
        actual_position: match controller.get_current_position(slave_id) {
            Ok(Some(pos)) => pos,
            _ => {
                log::error!("Failed to get actual position for slave {}", slave_id);
                return Err("Failed to get actual position".into());
            }
        },
        actual_velocity: match controller.get_current_velocity(slave_id) {
            Ok(Some(vel)) => vel,
            _ => {
                log::error!("Failed to get actual velocity for slave {}", slave_id);
                return Err("Failed to get actual velocity".into());
            }
        },
        actual_torque: match controller.get_current_torque(slave_id) {
            Ok(Some(torque)) => torque,
            _ => {
                log::error!("Failed to get actual torque for slave {}", slave_id);
                return Err("Failed to get actual torque".into());
            }
        },
        axis_sensors: match controller.get_current_axis_sensors(slave_id) {
            Ok(Some(sensor)) => sensor,
            _ => {
                log::error!("Failed to get axis sensor for slave {}", slave_id);
                return Err("Failed to get axis sensor".into());
            }
        },
        requested_target_position: match controller.get_current_target_position(slave_id) {
            Ok(Some(pos)) => pos,
            _ => {
                log::error!(
                    "Failed to get requested target position for slave {}",
                    slave_id
                );
                return Err("Failed to get requested target position".into());
            }
        },
        state: controller.get_status(slave_id) as u32,
        torque_state: match controller.is_torque_on(slave_id) {
            Ok(Some(state)) => state,
            _ => {
                log::error!("Failed to get torque state for slave {}", slave_id);
                return Err("Failed to get torque state".into());
            }
        },
        published_timestamp: Some(Timestamp::from(std::time::SystemTime::now())), 
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

        log::info!("New client - update period of {}s", request.get_ref().update_period);

        tokio::spawn(async move {
            let request = request.get_ref();
            let mut loop_timestamp = SystemTime::now();
            // state to be sent if no state is available
            // this is to avoid sending empty states
            let mut last_state = poule_empty_state();
            loop {
                let dt = request.update_period  - loop_timestamp.elapsed().unwrap().as_secs_f32() ;
                if dt > 0.0 {
                    sleep(Duration::from_secs_f32(dt)).await;
                }
                loop_timestamp = SystemTime::now();
                let states = PoulpeStates {
                    states: request
                        .ids
                        .iter()
                        .map(|&id| 
                            match get_state_for_id(&controller, id){
                                Ok(state) => {
                                    last_state = state.clone();
                                    state
                                },
                                Err(e) => {
                                    log::error!("Faile  d to get state for slave {}: {}", id, e);
                                    last_state.clone()
                                }
                            }

                        )
                        .collect(),
                };

                if tx.send(Ok(states)).await.is_err() {
                    break;
                }
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
        let mut dt_max :f32 = 0.0 ;
        let mut dropped_messages = 0;
        while let Some(Ok(req)) = stream.next().await {
            let t_loop = SystemTime::now();
            log::debug!("Got commands {:?}", req);
            for cmd in req.commands {
                let slave_id = cmd.id as u32;
                let mut target_pos = cmd.target_position;
                match cmd.published_timestamp{
                    Some(published_time) => {
                        let published_time = match SystemTime::try_from(published_time) {
                            Ok(systime) => systime,
                            Err(_) => {
                                log::warn!("Cannot parse the timestamp, discarding message!");
                                continue;
                            }
                        };
                        // check if the message is older than 5 ms
                        elapsed_time = published_time.elapsed().unwrap().as_millis();
                        if elapsed_time > 5 {
                            // log::warn!("Message older than {} ms, discarding!", 5);
                            dropped_messages +=1;
                            if cmd.compliancy.is_none() && cmd.velocity_limit.len()  == 0  && cmd.torque_limit.len() == 0 {
                                continue;
                            }
                            // target pos 
                            target_pos = vec!();
                        }
                    }
                    None => {
                        log::warn!("No published timestamp, discarding message!");
                        continue;
                    }
                }

                let no_axis = self.controller.get_orbita_type(slave_id) as usize;

                let mut set_compliant = cmd.compliancy;

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
            // wait for the next cycle  
            // to make sure the commands are executed
            // self.controller.inner.wait_for_next_cycle();

            nb += 1;
            command_times += elapsed_time;
            let dt = t.elapsed().unwrap().as_secs_f32();
            let dt_loop =  t_loop.elapsed().unwrap().as_secs_f32();
            if dt_max < dt_loop {
                dt_max = dt_loop;
            }
            if dt > 5.0 {
                let f = nb as f32 / dt ;
                let dt_c = (command_times as f32) / (nb as f32);
                log::info!("GRPC EtherCAT: {} req/s, dropped {:0.2} req/s, avg time: {} ms,  max {} ms", f, dropped_messages as f32/dt, dt_c, dt_max*1000.0);

                t = SystemTime::now();
                command_times = 0;
                nb = 0;
                dropped_messages = 0;
                dt_max = 0.0;
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


fn poule_empty_state() -> PoulpeState {
    PoulpeState {
        id: 0,
        compliant: false,
        actual_position: vec![],
        actual_velocity: vec![],
        actual_torque: vec![],
        axis_sensors: vec![],
        requested_target_position: vec![],
        state: 0,
        torque_state: false,
        published_timestamp: None,
    }
}