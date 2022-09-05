use std::time::Duration;

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

#[derive(Debug, Default)]
struct EposMultiplexerService {}

#[tonic::async_trait]
impl EposMultiplexer for EposMultiplexerService {
    async fn get_epos_ids(&self, request: Request<()>) -> Result<Response<EposIds>, Status> {
        let reply = EposIds { ids: vec![0, 1, 2] };

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
    let addr = "[::]:50098".parse()?;
    let srv = EposMultiplexerService::default();

    Server::builder()
        .add_service(EposMultiplexerServer::new(srv))
        .serve(addr)
        .await?;

    Ok(())
}
