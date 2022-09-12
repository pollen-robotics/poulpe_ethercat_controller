use std::{
    env,
    sync::Arc,
    time::{Duration, SystemTime},
};

use epos_ethercat_controller::EposController;
use tokio::{sync::mpsc, time::sleep};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{transport::Server, Request, Response, Status, Streaming};

use epos_ethercat_multiplexer::pb::{
    epos_multiplexer_server::{EposMultiplexer, EposMultiplexerServer},
    EposCommands, EposIds, EposState, EposStates, StateStreamRequest,
};

#[derive(Debug)]
struct EposMultiplexerService {
    controller: Arc<EposController>,
}

fn get_state_for_id(controller: &EposController, id: i32) -> EposState {
    let slave_id = id as u16;

    EposState {
        id,
        compliant: controller.is_on(slave_id),
        actual_position: controller.get_position_actual_value(slave_id),
        actual_velocity: controller.get_velocity_actual_value(slave_id) as f32,
        actual_torque: controller.get_torque_actual_value(slave_id) as f32,
        requested_target_position: controller.get_target_position(slave_id),
    }
}

#[tonic::async_trait]
impl EposMultiplexer for EposMultiplexerService {
    async fn get_epos_ids(&self, _request: Request<()>) -> Result<Response<EposIds>, Status> {
        let reply = EposIds {
            ids: self
                .controller
                .get_slave_ids()
                .iter()
                .map(|&id| id as i32)
                .collect(),
        };

        Ok(Response::new(reply))
    }

    type GetStatesStream = ReceiverStream<Result<EposStates, Status>>;

    async fn get_states(
        &self,
        request: Request<StateStreamRequest>,
    ) -> Result<Response<Self::GetStatesStream>, Status> {
        let (tx, rx) = mpsc::channel(4);

        let controller = self.controller.clone();

        tokio::spawn(async move {
            let request = request.get_ref();

            loop {
                let states = EposStates {
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
        request: Request<Streaming<EposCommands>>,
    ) -> Result<Response<()>, Status> {
        let mut stream = request.into_inner();

        let mut t = SystemTime::now();
        let mut nb = 0;

        while let Some(Ok(req)) = stream.next().await {
            log::debug!("Got commands {:?}", req);

            for cmd in req.commands {
                let slave_id = cmd.id as u16;

                if let Some(compliancy) = cmd.compliancy {
                    match compliancy {
                        false => self.controller.turn_on(slave_id, true),
                        true => self.controller.turn_off(slave_id),
                    }
                }

                if let Some(target_pos) = cmd.target_position {
                    self.controller
                        .set_target_position(slave_id, target_pos);
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

    let controller = EposController::connect(filename, 0_u32)?;

    for slave_id in controller.get_slave_ids() {
        log::info!("Setup Slave {}...", slave_id);
        controller.setup(slave_id);
        log::info!("Done!");
    }

    log::info!("EPOS controller ready!");

    let addr = "[::]:50098".parse()?;
    let srv = EposMultiplexerService {
        controller: Arc::new(controller),
    };

    Server::builder()
        .add_service(EposMultiplexerServer::new(srv))
        .serve(addr)
        .await?;

    Ok(())
}
