use std::{collections::HashMap, sync::Arc, time::Duration};

use super::pb::{
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
    TargetPosition(f32),
}

pub struct EposRemoteClient {
    rt: Runtime,

    state: Arc<RwLock<HashMap<u16, EposState>>>,
    command_buff: Arc<RwLock<HashMap<u16, Vec<Command>>>>,
}

impl EposRemoteClient {
    pub fn connect(addr: Uri, ids: Vec<u16>, update_period: Duration) -> Self {
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

    pub fn get_position_actual_value(&self, slave_id: u16) -> f32 {
        self.rt.block_on(self.state.read())[&slave_id].actual_position
    }

    pub fn get_velocity_actual_value(&self, slave_id: u16) -> i32 {
        self.rt.block_on(self.state.read())[&slave_id].actual_velocity as i32
    }

    pub fn get_torque_actual_value(&self, slave_id: u16) -> i16 {
        self.rt.block_on(self.state.read())[&slave_id].actual_torque as i16
    }

    pub fn is_on(&self, slave_id: u16) -> bool {
        self.rt.block_on(self.state.read())[&slave_id].compliant
    }

    pub fn get_target_position(&self, slave_id: u16) -> f32 {
        self.rt.block_on(self.state.read())[&slave_id].requested_target_position
    }

    pub fn turn_on(&mut self, slave_id: u16) {
        self.rt
            .block_on(self.command_buff.write())
            .entry(slave_id)
            .or_insert(vec![])
            .push(Command::Compliancy(false))
    }

    pub fn turn_off(&mut self, slave_id: u16) {
        self.rt
            .block_on(self.command_buff.write())
            .entry(slave_id)
            .or_insert(vec![])
            .push(Command::Compliancy(true))
    }

    pub fn set_target_position(&mut self, slave_id: u16, target_position: f32) {
        self.rt
            .block_on(self.command_buff.write())
            .entry(slave_id)
            .or_insert(vec![])
            .push(Command::TargetPosition(target_position))
    }
}

fn extract_commands(buff: &mut HashMap<u16, Vec<Command>>) -> Option<EposCommands> {
    if buff.is_empty() {
        return None;
    }

    let mut commands = vec![];

    for (&id, cmds) in buff.iter() {
        let mut epos_cmd = EposCommand {
            id: id.into(),
            ..Default::default()
        };

        for cmd in cmds {
            match cmd {
                Command::Compliancy(comp) => epos_cmd.compliancy = Some(*comp),
                Command::TargetPosition(pos) => epos_cmd.target_position = Some(*pos),
            }
        }

        commands.push(epos_cmd);
    }

    buff.clear();

    Some(EposCommands { commands })
}
