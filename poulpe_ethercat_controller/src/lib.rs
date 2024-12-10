use std::{
    collections::HashMap,
    error::Error,
    f32::consts::{E, PI},
    sync::RwLock,
    time::Duration,
};

pub mod state_machine;
use state_machine::{
    parse_homing_error_flags, parse_motor_error_flags, parse_state_from_status_bits,
    parse_status_word, CiA402State, ControlWord, ErrorFlags, StatusBit,
};

extern crate num;
#[macro_use]
extern crate num_derive;

use bitvec::prelude::*;

use ethercat_controller::{
    config::{PoulpeKind, SlaveConfig},
    Config, EtherCatController,
};

pub mod register;
use register::PdoRegister;

#[derive(Debug)]
pub struct PoulpeController {
    pub inner: EtherCatController,
    poulpe_config: HashMap<u16, PoulpeKind>,
}

impl PoulpeController {
    pub fn connect(filename: &str) -> Result<Self, Box<dyn Error>> {
        let config = Config::from_yaml(filename)?;

        let controller = EtherCatController::open(
            config.ethercat.master_id,
            Duration::from_micros(config.ethercat.cycle_time_us as u64),
            config.ethercat.command_drop_time_us,
            config.ethercat.watchdog_timeout_ms,
            config.ethercat.mailbox_wait_time_ms,
        )?
        .wait_for_ready();

        let mut poulpe_config = HashMap::new();

        for slave in config.slaves {
            if let SlaveConfig::Poulpe(poulpe) = slave {
                poulpe_config.insert(poulpe.id, poulpe);
            }
        }

        // if the feature allow_partial_network is enabled, we do not check if the slave is in the config file
        // if the feature is not enabled, we check if the slave is in the config file as the whole network should be defined in the config file
        #[cfg(not(feature = "allow_partial_network"))]
        {
            // check if all slaves are connected
            // check if the names of the slaves are correct
            let slave_ids = controller.get_slave_ids();
            for slave_id in slave_ids {
                if poulpe_config.get(&slave_id).is_none() {
                    log::error!(
                        "Slave {} with name {:?} not found in config, check config yaml file!",
                        slave_id,
                        controller.get_slave_name(slave_id).unwrap()
                    );
                    return Err("Slave not in yaml!".into());
                } else if let Some(name) = controller.get_slave_name(slave_id) {
                    if poulpe_config[&slave_id].name != name {
                        log::error!(
                            "Slave {} Name mismatch: expected {:?}, got {:?}",
                            slave_id,
                            poulpe_config[&slave_id].name,
                            name
                        );
                        return Err("Name mismatch".into());
                    } else {
                        log::error!("Slave {} with name {:?} name not found on EtherCAT network, check connection!", slave_id, poulpe_config[&slave_id].name);
                        return Err("Name not found, check connection!".into());
                    }
                }
            }

            // check if the number of slaves is correct
            let slave_ids = controller.get_slave_ids();
            let mut all_connected = true;
            if slave_ids.len() != poulpe_config.len() {
                for p in poulpe_config.keys() {
                    let name = controller.get_slave_name(*p);
                    if name.is_none() {
                        log::error!("Slave {} with name {:?} not found in Ethercat network, check connection!", p, poulpe_config[p].name);
                        all_connected = false;
                    }
                }
            }
            if all_connected == false {
                return Err("Number of slaves in config and in network do not match!".into());
            }
        }

        Ok(Self {
            inner: controller,
            poulpe_config,
        })
    }

    // function that checks if the time is longer that dropping time
    // returns true if its longer and false if not
    pub fn check_if_too_old(&self, message_ellased_time: Duration) -> bool {
        message_ellased_time.as_micros() as u32 > self.inner.command_drop_time_us
    }

    pub fn get_orbita_type(&self, id: u32) -> u32 {
        self.poulpe_config[&(id as u16)].orbita_type
    }

    pub fn get_slave_ids(&self) -> Vec<u32> {
        self.poulpe_config.keys().map(|x| *x as u32).collect()
    }

    pub fn get_slave_name(&self, slave_id: u16) -> Option<String> {
        self.poulpe_config.get(&slave_id).map(|x| x.name.clone())
    }

    pub fn get_slave_id(&self, name: &str) -> Option<u32> {
        for (id, poulpe) in &self.poulpe_config {
            if poulpe.name == name {
                return Some(*id as u32);
            }
        }
        None
    }

    pub fn get_slave_names(&self) -> Vec<String> {
        self.poulpe_config
            .values()
            .map(|x| x.name.clone())
            .collect()
    }

    pub fn is_slave_ready(&self, id: u16) -> bool {
        self.inner.is_slave_ready(id)
    }

    fn get_status_bits(&self, slave_id: u16) -> Result<Vec<StatusBit>, Box<dyn Error>> {
        let status_word = self.get_pdo_register(slave_id, PdoRegister::StatusWord, 0)?;
        let bits = u16::from_le_bytes(status_word.try_into().unwrap());
        Ok(parse_status_word(bits))
    }

    pub fn get_mode_of_operation(&self, slave_id: u16) -> Result<u8, Box<dyn Error>> {
        let mode_of_opearation = self.get_pdo_register(slave_id, PdoRegister::ModeOfOperation, 0);
        match mode_of_opearation {
            Ok(b) => Ok(b[0]),
            Err(_) => Err("Error reading mode of operation".into()),
        }
    }
    pub fn get_mode_of_operation_display(&self, slave_id: u16) -> Result<u8, Box<dyn Error>> {
        let mode_of_operation_display =
            self.get_pdo_register(slave_id, PdoRegister::ModeOfOperationDisplay, 0);
        match mode_of_operation_display {
            Ok(b) => Ok(b[0]),
            Err(_) => Err("Error reading mode of operation display".into()),
        }
    }

    fn clear_fault(&self, slave_id: u16) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn set_controlword(&self, slave_id: u16, value: u16) -> Result<(), Box<dyn Error>> {
        self.set_pdo_register(slave_id, PdoRegister::ControlWord, 0, &value.to_le_bytes())
    }

    pub fn get_error_flags(&self, slave_id: u16) -> Result<ErrorFlags, Box<dyn Error>> {
        let error_codes = self.get_pdo_registers(slave_id, PdoRegister::ErrorCode)?;
        let homing_error_flags = parse_homing_error_flags(error_codes[0][0..2].try_into().unwrap());
        let mut motor_error_flags = vec![Vec::new(); error_codes.len() - 1];
        for (i, e) in error_codes.iter().skip(1).enumerate() {
            motor_error_flags[i] = parse_motor_error_flags(e[0..2].try_into().unwrap());
        }

        Ok(ErrorFlags {
            motor_error_flags,
            homing_error_flags,
        })
    }

    fn wait_for_status_bit(
        &self,
        slave_id: u16,
        bit: StatusBit,
        timeout: Duration,
    ) -> Result<(), Box<dyn Error>> {
        let status_bits = self.get_status_bits(slave_id);
        log::debug!("Waiting for {:?} ({:?})", bit, status_bits);

        let start = std::time::Instant::now();
        loop {
            let status_bits = self.get_status_bits(slave_id)?;

            if status_bits.contains(&StatusBit::Fault) {
                // log::error!(
                //     "Slave {} in Fault state \n {:#x?}",
                //     slave_id,
                //     self.get_error_flags(slave_id)?,
                // );
                return Err("Fault status".into());
            }

            if status_bits.contains(&bit) {
                // bit is set
                break;
            }
            if start.elapsed() > timeout {
                log::error!("Timeout waiting for {:?} on slave {:?}", bit, slave_id);
                return Err("Timeout waiting for bit".into());
            }
            self.inner.wait_for_next_cycle();
        }
        Ok(())
    }

    fn get_pdo_register(
        &self,
        slave_id: u16,
        reg: PdoRegister,
        index: usize,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(self
            .inner
            .get_pdo_register(slave_id, &reg.name().to_string(), index)
            .unwrap())
    }
    fn set_pdo_register(
        &self,
        slave_id: u16,
        reg: PdoRegister,
        index: usize,
        value: &[u8],
    ) -> Result<(), Box<dyn Error>> {
        self.inner
            .set_pdo_register(slave_id, &reg.name().to_string(), index, value.to_vec());
        Ok(())
    }

    fn get_pdo_registers(
        &self,
        slave_id: u16,
        reg: PdoRegister,
    ) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        Ok(self
            .inner
            .get_pdo_registers(slave_id, &reg.name().to_string())
            .unwrap())
    }
    fn set_pdo_registers(
        &self,
        slave_id: u16,
        reg: PdoRegister,
        values: Vec<Vec<u8>>,
    ) -> Result<(), Box<dyn Error>> {
        self.inner
            .set_pdo_registers(slave_id, &reg.name().to_string(), values);
        Ok(())
    }
}

impl PoulpeController {
    pub fn setup(&self, id: u32) -> Result<(), Box<dyn Error>> {
        let slave_id = id as u16;

        // check if slave_id exists in etheract network
        if !self.inner.get_slave_ids().contains(&slave_id) {
            log::error!(
                "Slave {} with name {:?} not found in Ethercat network, check connection!",
                id,
                self.poulpe_config[&slave_id].name
            );
            return Err("Slave not connected!".into());
        }

        #[cfg(not(feature = "allow_partial_network"))]
        {
            // check if slave_id exists in config
            if !self.get_slave_ids().contains(&id) {
                log::error!(
                    "Slave {} with name {:?} not found in config, check config yaml file!",
                    id,
                    self.get_slave_name(slave_id).unwrap()
                );
                return Err("Slave not in yaml!".into());
            }
        }

        match self.inner.get_slave_name(slave_id) {
            Some(name) => {
                if self.poulpe_config[&slave_id].name != name {
                    log::error!(
                        "Slave {} Name mismatch: expected {:?}, got {:?}",
                        slave_id,
                        self.poulpe_config[&slave_id].name,
                        name
                    );
                    return Err("Name mismatch".into());
                }
            }
            _ => {
                log::error!("Slave {} name not found, check connection!", slave_id);
                return Err("Name not found, check connection!".into());
            }
        }

        // verify that the orbita type is the same as in the config file
        // orbita type is the number of axes
        // - orbita2s has 2 axes
        // - orbita3s has 3 axes
        #[cfg(feature = "verify_orbita_type")]
        {
            let no_axes = self.poulpe_config[&slave_id].orbita_type;
            let current_time = std::time::SystemTime::now();
            let orbita_type = loop {
                match self.get_type(slave_id as u32) {
                    0 => std::thread::sleep(std::time::Duration::from_millis(100)),
                    n => break n,
                }
                if current_time.elapsed().unwrap().as_millis() > 1000 {
                    log::error!(
                        "Slave {} - Commnunication not established:  1s timout on orbita type!",
                        id
                    );
                    return Err("Commnunication not established:  1s timout on orbita type!".into());
                }
            };
            if orbita_type != (no_axes as u8) {
                log::error!(
                    "Slave {} Orbita type mismatch: expected {}, got {}",
                    slave_id,
                    no_axes,
                    orbita_type
                );
                return Err("Orbita type mismatch".into());
            }
        }

        let state: CiA402State = self.get_status(slave_id as u32)?;
        log::info!("Slave {}, inital state: {:?}", slave_id, state);

        // get staus bits
        let mut status_bits = self.get_status_bits(slave_id)?;

        // verify if not in fault state
        if status_bits.contains(&StatusBit::Fault) {
            log::error!(
                "Slave {} in Fault state \n {:#x?}",
                slave_id,
                self.get_error_flags(slave_id)?,
            );


            #[cfg(not(feature = "allow_fault_on_slave"))]
            return Err("Fault status".into());
            #[cfg(feature = "allow_fault_on_slave")]
            {
                // turn off all the slaves if one of them is in fault state
                self.emergency_stop_all(slave_id)?;
                return Ok(());
            }
        }

        if state == CiA402State::NotReadyToSwitchOn {
            log::info!(
                "Slave {} in NotReadyToSwitchOn state, waiting to be rady for SwitchOn",
                slave_id
            );
            // wait 1s
            std::thread::sleep(std::time::Duration::from_secs(1));
            self.wait_for_status_bit(
                slave_id,
                StatusBit::SwitchedOnDisabled,
                Duration::from_secs(100),
            )?;

            // get staus bits
            status_bits = self.get_status_bits(slave_id)?;
        }

        // if enabled (should not be possible in normal operation)
        if status_bits.contains(&StatusBit::OperationEnabled) {
            #[cfg(feature = "turn_off_slaves_setup")]
            {
                // if the operation is enabled, we need
                // to disable it before we can set the controlword
                self.emergency_stop(id)?;
                log::warn!("Slave {} in OperationEnabled state, turning off", slave_id);
                self.wait_for_status_bit(slave_id, StatusBit::SwitchedOnDisabled, Duration::from_secs(20))?; // wait 20s (quick stop can take up to 10s)
                // get staus bits
                status_bits = self.get_status_bits(slave_id)?;
            }
            #[cfg(not(feature = "turn_off_slaves_setup"))]
            {
                log::info!("Slave {}, setup done! Current state: {:?}", slave_id, state);
                return Ok(());
            }
        }

        // if switch on disabled, we need to switch on
        if status_bits.contains(&StatusBit::SwitchedOnDisabled) {
            // go to the ready to switch on state
            self.set_controlword(slave_id, ControlWord::Shutdown.to_u16())?;
        }

        // here the slave shoulde be in ready to switch on state)
        self.wait_for_status_bit(slave_id, StatusBit::ReadyToSwitchOn, Duration::from_secs(1))?;
        status_bits = self.get_status_bits(slave_id)?;

        // if ready to switch on, we need to switch on
        if status_bits.contains(&StatusBit::ReadyToSwitchOn) {
            // go to the switched on state
            self.set_controlword(slave_id, ControlWord::SwitchOn.to_u16())?;
            self.wait_for_status_bit(slave_id, StatusBit::SwitchedOn, Duration::from_secs(1))?;
        }

        let state: CiA402State = self.get_status(slave_id as u32)?;
        log::info!("Slave {}, setup done! Current state: {:?}", slave_id, state);

        Ok(())
    }

    pub fn is_torque_on(&self, id: u32) -> Result<Option<bool>, Box<dyn std::error::Error>> {
        let slave_id = id as u16;
        let status = self.get_status_bits(slave_id)?;
        Ok(Some(status.contains(&StatusBit::OperationEnabled)))
    }

    pub fn set_torque(
        &self,
        id: u32,
        requested_torque: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let slave_id = id as u16;
        match self.is_torque_on(id) {
            Ok(Some(actual_torque)) => {
                log::debug!(
                    "Setting torque on slave {} to {} from {}",
                    id,
                    requested_torque,
                    actual_torque
                );
                let status_bits = self.get_status_bits(slave_id)?;
                if status_bits.contains(&StatusBit::Fault) {
                    #[cfg(feature = "allow_fault_on_slave")]
                    // return ok if the slave is in the fault state - dont try to set the torque
                    return Ok(());

                    // return error if the slave is in fault state - dont try to set the torque
                    log::error!("Slave {} in fault state", slave_id);
                    return Err("Slave in fault state status".into());
                }
                #[cfg(not(feature = "switchon_on_turnon"))]
                if status_bits.contains(&StatusBit::SwitchedOnDisabled) && requested_torque{
                    #[cfg(feature = "allow_fault_on_slave")]
                    // return ok if the slave is in switch on disabled state 
                    // the board is probably been turned off by a quick stop
                    return Ok(());

                    // return error if the slave is in fault state - dont try to set the torque
                    log::error!("Slave {} in SwitchedOnDisabled state, cannot enable torque!", slave_id);
                    return Err("Slave in SwitchedOnDisabled state status, cannot enable torque!".into());
                }

                if actual_torque == requested_torque {
                    return Ok(());
                } else {
                    // if turn on is requested, set the target position to the current position - safety feature
                    if requested_torque {
                        #[cfg(feature = "safe_turn_on")]
                        {
                            // set the target position to the current position
                            let current_position = self.get_current_position(id).unwrap().unwrap();
                            self.set_target_position(id, current_position.clone())
                                .unwrap();

                            // verify that the target position is set correctly and try 5 times
                            let mut target_position =
                                self.get_current_target_position(id).unwrap().unwrap();
                            let mut tries = 0;
                            // check if the target position is set correctly (small error margin)
                            while tries < 5
                                && (current_position
                                    .iter()
                                    .zip(target_position.iter())
                                    .all(|(a, b)| (a - b).abs() > 0.001))
                            {
                                self.set_target_position(id, current_position.clone())
                                    .unwrap();
                                std::thread::sleep(std::time::Duration::from_millis(2));
                                target_position =
                                    self.get_current_target_position(id).unwrap().unwrap();
                                tries += 1;
                            }
                            // throw error if the target position is not set correctly
                            if tries == 5 {
                                log::error!("Error setting target position!");
                                return Err("Error setting target position!".into());
                            }
                        }

                        #[cfg(feature = "switchon_on_turnon")]
                        {
                            // Switch on
                            self.set_controlword(slave_id, ControlWord::SwitchOn.to_u16())?;
                            self.wait_for_status_bit(
                                slave_id,
                                StatusBit::SwitchedOn,
                                Duration::from_secs(1),
                            )?;
                        }

                        // Enable
                        self.set_controlword(slave_id, ControlWord::EnableOperation.to_u16())?;
                        self.wait_for_status_bit(
                            slave_id,
                            StatusBit::OperationEnabled,
                            Duration::from_millis(5),
                        )?;
                    } else {
                        // Shutdown
                        self.set_controlword(slave_id, ControlWord::DisableOperation.to_u16())?;
                        self.wait_for_status_bit(
                            slave_id,
                            StatusBit::SwitchedOn,
                            Duration::from_millis(5),
                        )?;
                    }
                }
            }
            _ => {
                log::error!("Error getting torque state!");
                return Err("Error getting torque state!".into());
            }
        }
        Ok(())
    }

    fn get_pid(&self, _id: u32) -> Result<Option<(f32, f32, f32)>, Box<dyn std::error::Error>> {
        Ok(None)
    }
    fn set_pid(&self, _id: u32, _pid: (f32, f32, f32)) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    pub fn get_status(&self, slave_id: u32) -> Result<CiA402State, Box<dyn std::error::Error>> {
        let status_bits = self.get_status_bits(slave_id as u16)?;
        parse_state_from_status_bits(status_bits)
    }

    pub fn get_type(&self, slave_id: u32) -> u8 {
        let byte = match self.get_pdo_register(slave_id as u16, PdoRegister::ActuatorType, 0) {
            Ok(b) => b[0],
            Err(_) => 255,
        };
        byte
    }

    // make sure that the slave is disabled before changing the mode of operation
    pub fn set_mode_of_operation(&self, slave_id: u16, value: u8) -> Result<(), Box<dyn Error>> {
        // if the mode of operation is already set, return
        let mode_of_operation = self.get_mode_of_operation_display(slave_id)?;
        if mode_of_operation == value {
            return Ok(());
        }

        // if it is not verified that the slave is disabled
        // if not return error
        let is_on = self.is_torque_on(slave_id as u32)?;
        match is_on {
            Some(is_on) => match is_on {
                true => {
                    log::error!(
                        "Slave {} | Cannot change mode of operation when slave is turned on!",
                        slave_id
                    );
                    return Err("Cannot change mode of operation when torque is on!".into());
                }
                false => self.set_pdo_register(slave_id, PdoRegister::ModeOfOperation, 0, &[value]),
            },
            _ => {
                log::error!("Slave {} | Error getting torque state!", slave_id);
                return Err("Error getting torque state!".into());
            }
        }
    }

    fn get_register_values(
        &self,
        id: u32,
        register: PdoRegister,
    ) -> Result<Option<Vec<f32>>, Box<dyn std::error::Error>> {
        let slave_id = id as u16;
        match self.get_pdo_registers(slave_id, register) {
            Ok(bytes) => {
                let values = bytes
                    .iter()
                    .map(|x| f32::from_le_bytes(x[0..4].try_into().unwrap()))
                    .collect::<Vec<f32>>();
                Ok(Some(values))
            }
            Err(_) => Err("Error reading register!".into()),
        }
    }

    pub fn get_current_position(
        &self,
        id: u32,
    ) -> Result<Option<Vec<f32>>, Box<dyn std::error::Error>> {
        self.get_register_values(id, PdoRegister::PositionActualValue)
    }

    pub fn get_current_velocity(
        &self,
        id: u32,
    ) -> Result<Option<Vec<f32>>, Box<dyn std::error::Error>> {
        self.get_register_values(id, PdoRegister::VelocityActualValue)
    }

    pub fn get_current_torque(
        &self,
        id: u32,
    ) -> Result<Option<Vec<f32>>, Box<dyn std::error::Error>> {
        self.get_register_values(id, PdoRegister::TorqueActualValue)
    }

    pub fn get_current_axis_sensors(
        &self,
        id: u32,
    ) -> Result<Option<Vec<f32>>, Box<dyn std::error::Error>> {
        self.get_register_values(id, PdoRegister::AxisSensorActualValue)
    }

    pub fn get_current_target_position(
        &self,
        id: u32,
    ) -> Result<Option<Vec<f32>>, Box<dyn std::error::Error>> {
        self.get_register_values(id, PdoRegister::TargetPosition)
    }

    pub fn get_board_temperatures(
        &self,
        id: u32,
    ) -> Result<Option<Vec<f32>>, Box<dyn std::error::Error>> {
        self.get_register_values(id, PdoRegister::BoardTemperature)
    }

    pub fn get_motor_temperatures(
        &self,
        id: u32,
    ) -> Result<Option<Vec<f32>>, Box<dyn std::error::Error>> {
        self.get_register_values(id, PdoRegister::MotorTemperature)
    }

    pub fn get_temperatures(
        &self,
        id: u32,
    ) -> Result<Option<(Vec<f32>, Vec<f32>)>, Box<dyn std::error::Error>> {
        let board_temperatures = self.get_board_temperatures(id)?;
        let motor_temperatures = self.get_motor_temperatures(id)?;
        return match (board_temperatures, motor_temperatures) {
            (Some(b), Some(m)) => Ok(Some((b, m))),
            _ => Err("Error reading temperatures!".into()),
        };
    }

    // axis sensor zeros in firmware
    pub fn get_axis_sensor_zeros(
        &self,
        id: u32,
    ) -> Result<Option<Vec<f32>>, Box<dyn std::error::Error>> {
        self.get_register_values(id, PdoRegister::AxisZeroPosition)
    }

    // we are not actually reading it
    // we return the set value
    pub fn get_current_velocity_limit(
        &self,
        id: u32,
    ) -> Result<Option<Vec<f32>>, Box<dyn std::error::Error>> {
        self.get_register_values(id, PdoRegister::VelocityLimit)
    }

    // we are not actually reading it
    // we return the set value
    pub fn get_current_torque_limit(
        &self,
        id: u32,
    ) -> Result<Option<Vec<f32>>, Box<dyn std::error::Error>> {
        self.get_register_values(id, PdoRegister::TorqueLimit)
    }

    fn set_register_values(
        &self,
        id: u32,
        register: PdoRegister,
        values: Vec<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let slave_id = id as u16;
        let values_bytes: Vec<Vec<u8>> = values.iter().map(|&x| x.to_le_bytes().to_vec()).collect();
        self.set_pdo_registers(slave_id, register, values_bytes)
    }

    pub fn set_target_position(
        &self,
        id: u32,
        target_position: Vec<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_register_values(id, PdoRegister::TargetPosition, target_position)
    }

    pub fn set_velocity_limit(
        &self,
        id: u32,
        velocity_limit: Vec<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_register_values(id, PdoRegister::VelocityLimit, velocity_limit)
    }

    pub fn set_torque_limit(
        &self,
        id: u32,
        torque_limit: Vec<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_register_values(id, PdoRegister::TorqueLimit, torque_limit)
    }

    pub fn set_target_velocity(
        &self,
        id: u32,
        target_velocity: Vec<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_register_values(id, PdoRegister::TargetVelocity, target_velocity)
    }

    pub fn set_target_torque(
        &self,
        id: u32,
        target_torque: Vec<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_register_values(id, PdoRegister::TargetTorque, target_torque)
    }

    pub fn get_error_codes(&self, id: u32) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
        let slave_id = id as u16;
        match self.get_pdo_registers(slave_id, PdoRegister::ErrorCode) {
            Ok(bytes) => {
                let mut error_codes = Vec::new();
                for e in bytes.iter() {
                    error_codes.push(u16::from_le_bytes(e[0..2].try_into().unwrap()) as u32);
                }
                Ok(error_codes)
            }
            Err(_) => Err("Error reading error codes!".into()),
        }
    }

    pub fn emergency_stop(&self, id: u32) -> Result<(), Box<dyn std::error::Error>> {
        let slave_id = id as u16;
        self.set_controlword(slave_id, ControlWord::QuickStop.to_u16())
    }

    pub fn reactivate_after_emergency_stop(
        &self,
        id: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let slave_id = id as u16;
        self.set_controlword(slave_id, ControlWord::SwitchOn.to_u16())
    }

    // emergency stop on all slaves connected to the ethercat network
    pub fn emergency_stop_all(&self, slave_if_error_if: u16) -> Result<(), Box<dyn std::error::Error>> {
        for id in self.get_slave_ids() {
            self.emergency_stop(id as u32)?;
        }
        Ok(())
    }

}
