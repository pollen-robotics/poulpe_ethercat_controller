use std::{collections::HashMap, sync::Arc, time::Duration};

use super::pb::{
    poulpe_multiplexer_client::PoulpeMultiplexerClient, PoulpeCommand, PoulpeCommands, PoulpeState,
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
    TargetPosition(Vec<f32>),
    VelocityLimit(Vec<f32>),
    TorqueLimit(Vec<f32>),
}

pub struct PoulpeRemoteClient {
    rt: Runtime,

    state: Arc<RwLock<HashMap<u16, PoulpeState>>>,
    command_buff: Arc<RwLock<HashMap<u16, Vec<Command>>>>,
}

impl PoulpeRemoteClient {
    pub fn connect(addr: Uri, ids: Vec<u16>, update_period: Duration) -> Self {
        let state = Arc::new(RwLock::new(HashMap::new()));
        let state_lock = Arc::clone(&state);

        let command_buff = Arc::new(RwLock::new(HashMap::new()));
        let command_buff_lock = Arc::clone(&command_buff);

        let rt = Builder::new_multi_thread().enable_all().build().unwrap();

        let url = addr.to_string();

        rt.spawn(async move {
            let mut client = match PoulpeMultiplexerClient::connect(url).await {
                Ok(client) => client,
                Err(e) => {
                    log::error!(
                        "Error in connecting to the server! Check if server is up!!!\n  {:?}",
                        e
                    );
                    return;
                }
            };

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
            match client.get_commands(request).await {
                Ok(_) => log::info!("Command stream ended"),
                Err(e) => log::error!("Error in command stream: {:?}", e),
            }
        });

        let url = addr.to_string();

        rt.spawn(async move {
            let mut client = match PoulpeMultiplexerClient::connect(url).await {
                Ok(client) => client,
                Err(e) => {
                    log::error!(
                        "Error in connecting to the server! Check if server is up!!!\n  {:?}",
                        e
                    );
                    return;
                }
            };

            let request = Request::new(StateStreamRequest {
                ids: ids.iter().map(|&id| id as i32).collect(),
                update_period: update_period.as_secs_f32(),
            });

            let mut stream = client.get_states(request).await.unwrap().into_inner();
            while let Some(poulpe_state) = stream.message().await.unwrap() {
                log::debug!("Update state with {:?}", poulpe_state);
                {
                    let mut state_buff = state_lock.write().await;

                    for s in poulpe_state.states {
                        state_buff.insert(s.id as u16, s);
                    }
                }
            }
        });

        PoulpeRemoteClient {
            rt,
            state,
            command_buff,
        }
    }

    fn get_state_property<T, F>(&self, slave_id: u16, f: F, default: T) -> Result<T, ()>
    where
        F: Fn(&PoulpeState) -> T,
    {
        match self.rt.block_on(self.state.read()).get(&slave_id) {
            Some(state) => Ok(f(state)),
            None => {
                log::error!("No state found for slave {}", slave_id);
                Err(())
            }
        }
    }

    // adding the state properties to the client
    // for vector states make sure to use clone() to avoid borrowing issues
    pub fn get_position_actual_value(&self, slave_id: u16) -> Result<Vec<f32>, ()> {
        self.get_state_property(slave_id, |state| state.actual_position.clone(), vec![])
    }

    pub fn get_velocity_actual_value(&self, slave_id: u16) -> Result<Vec<f32>, ()> {
        self.get_state_property(slave_id, |state| state.actual_velocity.clone(), vec![])
    }

    pub fn get_torque_actual_value(&self, slave_id: u16) -> Result<Vec<f32>, ()> {
        self.get_state_property(slave_id, |state| state.actual_torque.clone(), vec![])
    }

    pub fn is_on(&self, slave_id: u16) -> Result<bool, ()> {
        self.get_state_property(slave_id, |state| state.torque_state, false)
    }

    pub fn get_target_position(&self, slave_id: u16) -> Result<Vec<f32>, ()> {
        self.get_state_property(
            slave_id,
            |state| state.requested_target_position.clone(),
            vec![],
        )
    }

    pub fn get_state(&self, slave_id: u16) -> Result<u32, ()> {
        self.get_state_property(slave_id, |state| state.state, 255)
    }

    pub fn get_torque_state(&self, slave_id: u16) -> Result<bool, ()> {
        self.get_state_property(slave_id, |state| state.torque_state, false)
    }

    pub fn get_axis_sensors(&self, slave_id: u16) -> Result<Vec<f32>, ()> {
        self.get_state_property(slave_id, |state| state.axis_sensors.clone(), vec![])
    }

    fn push_command(&mut self, slave_id: u16, command: Command) {
        self.rt
            .block_on(self.command_buff.write())
            .entry(slave_id)
            .or_insert_with(Vec::new)
            .push(command);
    }

    pub fn turn_on(&mut self, slave_id: u16) {
        self.push_command(slave_id, Command::Compliancy(false));
    }

    pub fn turn_off(&mut self, slave_id: u16) {
        self.push_command(slave_id, Command::Compliancy(true));
    }

    pub fn set_target_position(&mut self, slave_id: u16, target_position: Vec<f32>) {
        self.push_command(slave_id, Command::TargetPosition(target_position));
    }
    pub fn set_velocity_limit(&mut self, slave_id: u16, velocity_limit: Vec<f32>) {
        self.push_command(slave_id, Command::VelocityLimit(velocity_limit));
    }
    pub fn set_torque_limit(&mut self, slave_id: u16, torque_limit: Vec<f32>) {
        self.push_command(slave_id, Command::TorqueLimit(torque_limit));
    }
}

fn extract_commands(buff: &mut HashMap<u16, Vec<Command>>) -> Option<PoulpeCommands> {
    if buff.is_empty() {
        return None;
    }

    let mut commands = vec![];

    for (&id, cmds) in buff.iter() {
        let mut poulpe_cmd = PoulpeCommand {
            id: id.into(),
            ..Default::default()
        };

        for cmd in cmds {
            match cmd {
                Command::Compliancy(comp) => poulpe_cmd.compliancy = Some(*comp),
                Command::TargetPosition(pos) => {
                    if pos.len() != 0 {
                        poulpe_cmd.target_position.extend(pos.iter().cloned());
                    }
                }
                Command::VelocityLimit(vel) => {
                    if vel.len() != 0 {
                        poulpe_cmd.velocity_limit.extend(vel.iter().cloned());
                    }
                }
                Command::TorqueLimit(torque) => {
                    if torque.len() != 0 {
                        poulpe_cmd.torque_limit.extend(torque.iter().cloned());
                    }
                }
            }
        }
        commands.push(poulpe_cmd);
    }

    buff.clear();

    Some(PoulpeCommands { commands })
}
