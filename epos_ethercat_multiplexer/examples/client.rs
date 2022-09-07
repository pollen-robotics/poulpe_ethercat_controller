use std::{
    collections::HashMap,
    error::Error,
    sync::Arc,
    time::{Duration, SystemTime},
};

use epos_ethercat_multiplexer::pb::{
    epos_multiplexer_client::EposMultiplexerClient, EposCommand, EposCommands, StateStreamRequest,
};
use tokio::{
    sync::{
        mpsc::{self, Sender},
        RwLock,
    },
    time::sleep, runtime::{Builder, Runtime},
};
use tonic::{
    codegen::http::request,
    transport::{Channel, Uri},
    Request,
};

#[derive(Debug)]
enum Command {
    Compliancy(bool),
    TargetPosition(u32),
}

struct EposRemoteClient {
    command_buff: Arc<RwLock<HashMap<u16, Vec<Command>>>>,
}

impl EposRemoteClient {
    async fn connect(addr: Uri, update_period: Duration) -> Result<Self, Box<dyn Error>> {
        let command_buff = Arc::new(RwLock::new(HashMap::new()));
        let command_buff_lock = Arc::clone(&command_buff);

        let client = EposRemoteClient { command_buff };

        let mut inner = EposMultiplexerClient::connect(addr).await?;

        let command_stream = async_stream::stream! {
            loop {
                {
                    let mut cmd_map = command_buff_lock.write().await;
                    let commands = extract_commands(&mut cmd_map);
                    yield commands;
                }

                sleep(update_period).await;
            }
        };

        let request = Request::new(command_stream);
        inner.get_commands(request).await.unwrap();

        Ok(client)
    }

    async fn set_target_position(&mut self, slave_id: u16, target_position: u32) {

        self.command_buff
            .write()
            .await
            .entry(slave_id)
            .or_insert(vec![])
            .push(Command::TargetPosition(target_position))
    }
}

fn extract_commands(buff: &mut HashMap<u16, Vec<Command>>) -> EposCommands {
    let mut commands = vec![];

    for (&id, cmds) in buff.iter() {
        let mut epos_cmd = EposCommand::default();
        epos_cmd.id = id.into();

        for cmd in cmds {
            match cmd {
                Command::Compliancy(comp) => epos_cmd.compliancy = Some(*comp),
                Command::TargetPosition(pos) => epos_cmd.target_position = Some(*pos as f32),
            }
        }

        commands.push(epos_cmd);
    }

    EposCommands { commands }
}

async fn get_states(client: &mut EposMultiplexerClient<Channel>) -> Result<(), Box<dyn Error>> {
    let request = Request::new(StateStreamRequest {
        ids: vec![0],
        update_period: 0.001,
    });
    let mut stream = client.get_states(request).await?.into_inner();

    let mut t = SystemTime::now();

    while let Some(state) = stream.message().await? {
        println!("{:?} {:?}", state.states[0].actual_position, t.elapsed());
        t = SystemTime::now();
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut client = EposMultiplexerClient::connect("http://127.0.0.1:50098").await?;

    let ids = client.get_epos_ids(()).await;
    println!("Got ids {:?}", ids);

    get_states(&mut client).await?;

    Ok(())
}
