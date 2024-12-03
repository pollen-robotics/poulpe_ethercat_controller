use std::{any::Any, collections::HashMap, f32::consts::E, sync::Arc, time::Duration};

use super::pb::{
    poulpe_multiplexer_client::PoulpeMultiplexerClient, PoulpeCommand, PoulpeCommands, PoulpeState,
    StateStreamRequest,
};
use prost_types::Timestamp;
use tokio::{
    runtime::{Builder, Handle, Runtime},
    sync::RwLock,
    time::sleep,
};
use tonic::{transport::Uri, Request};

use poulpe_ethercat_controller::register::BoardStatus;

#[derive(Debug)]
enum Command {
    EmergencyStop(bool),
    Compliancy(bool),
    ModeOfOperation(u32),
    TargetPosition(Vec<f32>),
    TargetVelocity(Vec<f32>),
    TargetTorque(Vec<f32>),
    VelocityLimit(Vec<f32>),
    TorqueLimit(Vec<f32>),
}

#[derive(Debug)]
pub struct PoulpeRemoteClient {
    ids: Vec<u16>,
    rt: Arc<Runtime>,
    addr: Uri,
    state: Arc<RwLock<HashMap<u16, PoulpeState>>>,
    command_buff: Arc<RwLock<HashMap<u16, Vec<Command>>>>,
}

impl PoulpeRemoteClient {
    pub fn connect(
        addr: Uri,
        poulpe_ids: Vec<u16>,
        update_period: Duration,
    ) -> Result<Self, std::io::Error> {
        let state = Arc::new(RwLock::new(HashMap::new()));
        let state_lock = Arc::clone(&state);

        let command_buff = Arc::new(RwLock::new(HashMap::new()));
        let command_buff_lock = Arc::clone(&command_buff);

        // let rt = Builder::new_multi_thread().enable_all().build().unwrap();
        let rt = Arc::new(Builder::new_multi_thread().enable_all().build().unwrap());

        // Validate poulpe_ids
        let client = PoulpeRemoteClient {
            ids: poulpe_ids.clone(),
            rt: rt.clone(),
            addr: addr.clone(),
            state: state.clone(),
            command_buff: command_buff.clone(),
        };

        let url = addr.to_string();
        let ids = poulpe_ids.clone();
        // check if the id is valid and the server is up
        // check if poulpe_ids are valid
        match client.get_poulpe_ids_sync() {
            Ok(available_ids) => {
                let available_ids = available_ids.0;
                let mut common_ids = available_ids.clone();
                common_ids.retain(|id| poulpe_ids.contains(id));
                if common_ids.len() != poulpe_ids.len() {
                    log::error!(
                        "Invalid poulpe_ids: {:?}, available_ids: {:?}",
                        poulpe_ids,
                        available_ids
                    );
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Invalid poulpe_ids",
                    ));
                }
            }
            Err(e) => {
                log::error!(
                    "Error in connecting to the server! Check if server is up!!!\n  {:?}",
                    e
                );
                return Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "Error in connecting to the server! Check if server is up!!!",
                ));
            }
        }

        // spawn a single thread to handle both the state stream and command stream
        rt.spawn(async move {
            // Connect to the server
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

            // Prepare the state stream request
            let state_request = Request::new(StateStreamRequest {
                ids: poulpe_ids.iter().map(|&id| id as i32).collect(),
                update_period: update_period.as_secs_f32(),
            });

            // Start receiving states
            let mut state_stream = client.get_states(state_request).await.unwrap().into_inner();

            // Prepare the command stream
            let command_stream = async_stream::stream! {
                // fixed frequency
                let mut interval = tokio::time::interval(update_period / 2);

                loop {
                    // next cycle
                    interval.tick().await;
                    let mut cmd_map = command_buff_lock.write().await;
                    if let Some(commands) = extract_commands(&mut cmd_map) {
                        yield commands;
                    }
                }
            };

            // Send commands in parallel with state handling
            tokio::select! {
                // Handle state stream
                _ = async {
                    while let Some(poulpe_state) = state_stream.message().await.unwrap() {
                        log::debug!("Update state with {:?}", poulpe_state);
                        let mut state_buff = state_lock.write().await;
                        for s in poulpe_state.states {
                            state_buff.insert(s.id as u16, s);
                        }
                    }
                } => {},

                // Handle command stream
                result = client.get_commands(Request::new(command_stream)) => {
                    match result {
                        Ok(_) => log::info!("Command stream ended"),
                        Err(e) => log::error!("Error in command stream: {:?}", e),
                    }
                },
            }
        });

        Ok(PoulpeRemoteClient {
            ids,
            rt,
            addr,
            state,
            command_buff,
        })
    }

    pub fn get_poulpe_ids_sync(
        &self,
    ) -> Result<(Vec<u16>, Vec<String>), Box<dyn std::error::Error>> {
        self.rt.block_on(async {
            let mut client = PoulpeMultiplexerClient::connect(self.addr.to_string()).await?;
            get_poulpe_ids_async(&mut client).await
        })
    }

    pub fn get_poulpe_ids(&self) -> Vec<u16> {
        self.rt
            .block_on(self.state.read())
            .keys()
            .cloned()
            .collect()
    }

    // get the state property
    // check if the state is older than 1s
    fn get_state_property<T, F>(&self, slave_id: u16, f: F, _default: T) -> Result<T, ()>
    where
        F: Fn(&PoulpeState) -> T,
    {
        let state = self.rt.block_on(self.state.read());
        let state = state.get(&slave_id).ok_or_else(|| {
            log::error!("No state found for slave {}", slave_id);
        })?;

        if let Some(ts) = &state.published_timestamp {
            if let Ok(systime) = std::time::SystemTime::try_from(ts.clone()) {
                if systime.elapsed().unwrap().as_millis() > 1000 {
                    log::error!(
                        "State is older than 1s for slave {}, server maybe down!",
                        slave_id
                    );
                    // kill the slave if error recovery not supported
                    #[cfg(not(feature = "recover_from_error"))]
                    std::process::exit(10);
                    return Err(());
                }
            } else {
                log::warn!("Cannot parse the timestamp, discarding message!");
                // kill the slave if error recovery not supported
                #[cfg(not(feature = "recover_from_error"))]
                std::process::exit(10);
                return Err(());
            }
        } else {
            log::warn!("No timestamp found for slave {}", slave_id);
        }

        Ok(f(state))
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
        self.get_state_property(slave_id, |state| state.compliant, false)
    }

    pub fn get_target_position(&self, slave_id: u16) -> Result<Vec<f32>, ()> {
        self.get_state_property(
            slave_id,
            |state| state.requested_target_position.clone(),
            vec![],
        )
    }

    pub fn get_motor_temperatures(&self, slave_id: u16) -> Result<Vec<f32>, ()> {
        self.get_state_property(slave_id, |state| state.motor_temperatures.clone(), vec![])
    }
    pub fn get_board_temperatures(&self, slave_id: u16) -> Result<Vec<f32>, ()> {
        self.get_state_property(slave_id, |state| state.board_temperatures.clone(), vec![])
    }

    pub fn get_mode_of_operation(&self, slave_id: u16) -> Result<u32, ()> {
        self.get_state_property(slave_id, |state| state.mode_of_operation as u32, 255)
    }

    pub fn get_state(&self, slave_id: u16) -> Result<u32, ()> {
        // temporaty solution trasnforming the state to board status
        match BoardStatus::from_cia402_to_board_status(
            self.get_state_property(slave_id, |state| state.state, 255)?,
            self.get_state_property(slave_id, |state| state.error_codes.clone(), vec![])?,
        ) {
            Ok(s) => Ok(s as u32),
            Err(_) => Err(()),
        }
        // self.get_state_property(slave_id, |state| state.state, 255)
    }

    pub fn get_cia402_state(&self, slave_id: u16) -> Result<u32, ()> {
        self.get_state_property(slave_id, |state| state.state, 255)
    }

    pub fn get_torque_state(&self, slave_id: u16) -> Result<bool, ()> {
        self.get_state_property(slave_id, |state| state.compliant, false)
    }

    pub fn get_axis_sensors(&self, slave_id: u16) -> Result<Vec<f32>, ()> {
        self.get_state_property(slave_id, |state| state.axis_sensors.clone(), vec![])
    }

    pub fn get_axis_sensor_zeros(&self, slave_id: u16) -> Result<Vec<f32>, ()> {
        self.get_state_property(slave_id, |state| state.axis_sensor_zeros.clone(), vec![])
    }

    pub fn get_error_codes(&self, slave_id: u16) -> Result<Vec<i32>, ()> {
        self.get_state_property(slave_id, |state| state.error_codes.clone(), vec![])
    }

    pub fn get_velocity_limit(&self, slave_id: u16) -> Result<Vec<f32>, ()> {
        self.get_state_property(
            slave_id,
            |state| state.requested_velocity_limit.clone(),
            vec![],
        )
    }
    pub fn get_torque_limit(&self, slave_id: u16) -> Result<Vec<f32>, ()> {
        self.get_state_property(
            slave_id,
            |state| state.requested_torque_limit.clone(),
            vec![],
        )
    }

    fn push_command(&mut self, slave_id: u16, command: Command) -> Result<(), ()> {
        self.rt
            .block_on(self.command_buff.write())
            .entry(slave_id)
            .or_insert_with(Vec::new)
            .push(command);
        Ok(())
    }

    pub fn turn_on(&mut self, slave_id: u16) {
        self.push_command(slave_id, Command::Compliancy(false));
    }

    pub fn turn_off(&mut self, slave_id: u16) {
        self.push_command(slave_id, Command::Compliancy(true));
    }

    pub fn set_mode_of_operation(&mut self, slave_id: u16, mode: u32) {
        self.push_command(slave_id, Command::ModeOfOperation(mode));
    }

    pub fn set_target_position(&mut self, slave_id: u16, target_position: Vec<f32>) {
        self.push_command(slave_id, Command::TargetPosition(target_position));
    }
    pub fn set_target_velocity(&mut self, slave_id: u16, target_velocity: Vec<f32>) {
        self.push_command(slave_id, Command::TargetVelocity(target_velocity));
    }
    pub fn set_target_torque(&mut self, slave_id: u16, target_torque: Vec<f32>) {
        self.push_command(slave_id, Command::TargetTorque(target_torque));
    }

    pub fn set_velocity_limit(&mut self, slave_id: u16, velocity_limit: Vec<f32>) {
        self.push_command(slave_id, Command::VelocityLimit(velocity_limit));
    }
    pub fn set_torque_limit(&mut self, slave_id: u16, torque_limit: Vec<f32>) {
        self.push_command(slave_id, Command::TorqueLimit(torque_limit));
    }

    pub fn emergency_stop(&mut self, slave_id: u16) {
        self.push_command(slave_id, Command::EmergencyStop(true));
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
                Command::EmergencyStop(stop) => poulpe_cmd.emergency_stop = Some(*stop),
                Command::Compliancy(comp) => poulpe_cmd.compliancy = Some(*comp),
                Command::ModeOfOperation(mode) => poulpe_cmd.mode_of_operation = *mode as i32,
                Command::TargetPosition(pos) => {
                    if pos.len() != 0 {
                        poulpe_cmd.target_position.extend(pos.iter().cloned());
                    }
                }
                Command::TargetVelocity(vel) => {
                    if vel.len() != 0 {
                        poulpe_cmd.target_velocity.extend(vel.iter().cloned());
                    }
                }
                Command::TargetTorque(torque) => {
                    if torque.len() != 0 {
                        poulpe_cmd.target_torque.extend(torque.iter().cloned());
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
        poulpe_cmd.published_timestamp = Some(Timestamp::from(std::time::SystemTime::now()));
        commands.push(poulpe_cmd);
    }

    buff.clear();

    Some(PoulpeCommands { commands })
}

pub async fn get_poulpe_ids_async(
    client: &mut PoulpeMultiplexerClient<tonic::transport::Channel>,
) -> Result<(Vec<u16>, Vec<String>), Box<dyn std::error::Error>> {
    let response = client.get_poulpe_ids(Request::new(())).await?;
    let response = response.into_inner();
    let ids: Vec<u16> = response.ids.into_iter().map(|id| id as u16).collect();
    let names: Vec<String> = response
        .names
        .into_iter()
        .map(|name: String| name as String)
        .collect();
    Ok((ids, names))
}
