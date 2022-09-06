use std::{env, time::Duration};

use epos_ethercat_controller::EposController;
use tokio::{sync::mpsc, time::sleep};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{transport::Server, Request, Response, Status, Streaming};

use pb::{
    epos_multiplexer_server::{EposMultiplexer, EposMultiplexerServer},
    Commands, EposIds, EposState, StateStreamRequest,
};

pub mod pb {
    tonic::include_proto!("epos");
}

#[derive(Debug)]
struct EposMultiplexerService {
    controller: EposController,
}

#[tonic::async_trait]
impl EposMultiplexer for EposMultiplexerService {
    async fn get_epos_ids(&self, request: Request<()>) -> Result<Response<EposIds>, Status> {
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

    type GetStatesStream = ReceiverStream<Result<EposState, Status>>;

    async fn get_states(
        &self,
        request: Request<StateStreamRequest>,
    ) -> Result<Response<Self::GetStatesStream>, Status> {
        let (tx, rx) = mpsc::channel(4);

        tokio::spawn(async move {
            let request = request.get_ref();

            while tx.send(Ok(EposState::default())).await.is_ok() {
                sleep(Duration::from_secs_f32(request.update_period)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_commands(
        &self,
        request: Request<Streaming<Commands>>,
    ) -> Result<Response<()>, Status> {
        let mut stream = request.into_inner();

        while let Some(cmd) = stream.next().await {
            println!("{:?}", cmd);
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

    let addr = "[::]:50098".parse()?;
    let srv = EposMultiplexerService { controller };

    Server::builder()
        .add_service(EposMultiplexerServer::new(srv))
        .serve(addr)
        .await?;

    Ok(())
}
