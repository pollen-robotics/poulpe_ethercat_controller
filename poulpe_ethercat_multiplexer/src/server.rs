use std::{
    env,
    f32::consts::E,
    mem::take,
    sync::Arc,
    time::{Duration, SystemTime},
};

use poulpe_ethercat_controller::PoulpeController;
use tokio::{sync::mpsc, time::sleep};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{transport::Server, Request, Response, Status, Streaming};

use poulpe_ethercat_multiplexer::pb::{
    poulpe_multiplexer_server::{PoulpeMultiplexer, PoulpeMultiplexerServer},
    PoulpeCommands, PoulpeIds, PoulpeState, PoulpeStates, StateStreamRequest,
};

#[derive(Debug)]
struct PoulpeMultiplexerService {
    controller: Arc<PoulpeController>,
}

fn get_state_for_id(controller: &PoulpeController, id: i32) -> PoulpeState {
    let slave_id = id as u32;

    PoulpeState {
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
    }
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
                        .map(|&id| get_state_for_id(&controller, id))
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

        while let Some(Ok(req)) = stream.next().await {
            log::debug!("Got commands {:?}", req);
            for cmd in req.commands {
                let slave_id = cmd.id as u32;

                if let Some(compliancy) = cmd.compliancy {
                    match compliancy {
                        false => self.controller.set_torque(slave_id, true).unwrap(),
                        true => self.controller.set_torque(slave_id, false).unwrap(),
                    }
                }

                let target_pos = cmd.target_position;
                if target_pos.len() != 0 {
                    self.controller.set_target_position(slave_id, target_pos);
                }
                let velocity_limit = cmd.velocity_limit;
                if velocity_limit.len() != 0 {
                    self.controller.set_velocity_limit(slave_id, velocity_limit);
                }
                let torque_limit = cmd.torque_limit;
                if torque_limit.len() != 0 {
                    self.controller.set_torque_limit(slave_id, torque_limit);
                }
            }

            nb += 1;

            let dt = t.elapsed().unwrap().as_secs_f32();
            if dt > 1.0 {
                let f = nb as f32 / dt;
                log::info!("Got {} req/s", f);

                t = SystemTime::now();
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
        controller.setup(slave_id);
        log::info!("Done!");
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
