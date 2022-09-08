use std::{
    collections::HashMap,
    env,
    error::Error,
    f32::consts::PI,
    sync::Arc,
    thread,
    time::{Duration, SystemTime},
};

use epos_ethercat_multiplexer::pb::{
    epos_multiplexer_client::EposMultiplexerClient, EposCommand, EposCommands, EposState,
    StateStreamRequest,
};
use tokio::{
    runtime::{Builder, Runtime},
    sync::RwLock,
    time::sleep,
};
use tonic::{transport::Uri, Request};

#[derive(Debug)]
enum Command {
    Compliancy(bool),
    TargetPosition(u32),
}

struct EposRemoteClient {
    rt: Runtime,

    state: Arc<RwLock<HashMap<u16, EposState>>>,
    command_buff: Arc<RwLock<HashMap<u16, Vec<Command>>>>,
}

impl EposRemoteClient {
    fn connect(addr: Uri, ids: Vec<u16>, update_period: Duration) -> Self {
        let state = Arc::new(RwLock::new(HashMap::new()));
        let state_lock = Arc::clone(&state);

        let command_buff = Arc::new(RwLock::new(HashMap::new()));
        let command_buff_lock = Arc::clone(&command_buff);

        let rt = Builder::new_multi_thread().enable_all().build().unwrap();

        let url = addr.to_string();

        rt.spawn(async move {
            let mut client = EposMultiplexerClient::connect(url).await.unwrap();

            let command_stream = async_stream::stream! {
                loop {
                    {
                        let mut cmd_map = command_buff_lock.write().await;
                        if let Some(commands) = extract_commands(&mut cmd_map) {
                            yield commands;
                        }
                    }

                    sleep(update_period).await;
                }
            };
            let request = Request::new(command_stream);
            client.get_commands(request).await.unwrap();
        });

        let url = addr.to_string();

        rt.spawn(async move {
            let mut client = EposMultiplexerClient::connect(url).await.unwrap();

            let request = Request::new(StateStreamRequest {
                ids: ids.iter().map(|&id| id as i32).collect(),
                update_period: update_period.as_secs_f32(),
            });

            let mut stream = client.get_states(request).await.unwrap().into_inner();
            while let Some(epos_state) = stream.message().await.unwrap() {
                log::debug!("Update state with {:?}", epos_state);
                {
                    let mut state_buff = state_lock.write().await;

                    for s in epos_state.states {
                        state_buff.insert(s.id as u16, s);
                    }
                }
            }
        });

        EposRemoteClient {
            rt,
            state,
            command_buff,
        }
    }

    fn turn_on(&mut self, slave_id: u16) {
        self.rt
            .block_on(self.command_buff.write())
            .entry(slave_id)
            .or_insert(vec![])
            .push(Command::Compliancy(false))
    }

    fn turn_off(&mut self, slave_id: u16) {
        self.rt
            .block_on(self.command_buff.write())
            .entry(slave_id)
            .or_insert(vec![])
            .push(Command::Compliancy(true))
    }

    fn set_target_position(&mut self, slave_id: u16, target_position: u32) {
        self.rt
            .block_on(self.command_buff.write())
            .entry(slave_id)
            .or_insert(vec![])
            .push(Command::TargetPosition(target_position))
    }

    fn get_position_actual_value(&self, slave_id: u16) -> u32 {
        self.rt.block_on(self.state.read())[&slave_id].actual_position as u32
    }
}

fn extract_commands(buff: &mut HashMap<u16, Vec<Command>>) -> Option<EposCommands> {
    if buff.is_empty() {
        return None;
    }

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

    buff.clear();

    Some(EposCommands { commands })
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let passiv_id = 2;
    let active_id = 1;

    let mut client =
        EposRemoteClient::connect("http://127.0.0.1:50098".parse()?, vec![active_id, passiv_id], Duration::from_millis(1));

    log::info!("Turn off slave {}", passiv_id);
    client.turn_off(passiv_id);

    log::info!("Turn on slave {}", active_id);
    client.turn_on(active_id);

    let t0 = SystemTime::now();

    let offset = 1000.0;
    let amp = 1000.0;
    let freq = 0.5;

    thread::sleep(Duration::from_secs(1));

    loop {
        let actual_position = client.get_position_actual_value(passiv_id);

        let t = t0.elapsed().unwrap().as_secs_f32();
        let target_position = offset + amp * (2.0 * PI * freq * t).sin();
        let target_position = target_position as u32;

        log::info!("Pos: {} Target: {}", actual_position, target_position);

        client.set_target_position(active_id, actual_position);
        thread::sleep(Duration::from_millis(1));
    }
}
