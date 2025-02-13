use std::{collections::HashMap, sync::Arc, time::Duration};

use poulpe_ethercat_grpc::client;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use tonic::transport::Uri;

use poulpe_ethercat_grpc::client::{PoulpeIdClient, PoulpeRemoteClient};

use poulpe_ethercat_controller::state_machine;

#[pyclass]
pub struct PyPoulpeRemoteClient {
    client: PoulpeRemoteClient,
}

#[pymethods]
impl PyPoulpeRemoteClient {
    #[new]
    pub fn new(addr: &str, ids: Vec<u16>, update_period: f32) -> Self {
        let addr_uri = match addr.parse::<Uri>() {
            Ok(uri) => uri,
            Err(_) => panic!("Invalid URI format"),
        };
        let duration = Duration::from_secs_f32(update_period);

        let client = match PoulpeRemoteClient::connect(addr_uri, ids, duration) {
            Ok(client) => client,
            Err(e) => panic!("Failed to connect to the server: {}", e),
        };

        PyPoulpeRemoteClient { client }
    }

    /// Get the mode of operation
    ///
    /// Args:
    ///     slave_id (int): The slave id
    /// Returns:
    ///     int: The mode of operation  - 1: Profile Position Mode, 3: Profile Velocity Mode, 4: Profile Torque Mode
    pub fn get_mode_of_operation(&mut self, slave_id: u16) -> u32 {
        match self.client.get_mode_of_operation(slave_id) {
            Ok(mode) => mode,
            _ => panic!("Error in getting mode of operation"),
        }
    }

    /// Set the mode of operation
    ///
    /// Args:
    ///     slave_id (int): The slave id
    ///     mode (int): The mode of operation  - 1: Profile Position Mode, 3: Profile Velocity Mode, 4: Profile Torque Mode
    pub fn set_mode_of_operation(&mut self, slave_id: u16, mode: u32) {
        self.client.set_mode_of_operation(slave_id, mode);
    }

    /// Print the mode of operation
    ///
    /// Args:
    ///     slave_id (int): The slave id
    ///
    /// Outputs the mode of operation
    pub fn print_mode_of_operation(&mut self, slave_id: u16) {
        let mode = match self.client.get_mode_of_operation(slave_id) {
            Ok(mode) => mode,
            _ => panic!("Error in getting mode of operation"),
        };
        let mop = state_machine::CiA402ModeOfOperation::from_u8(mode as u8).unwrap();
        println!("Mode of operation: {:?}", mop);
    }

    /// Enable the actuators
    ///
    /// Args:
    ///    slave_id (int): The slave id
    pub fn turn_on(&mut self, slave_id: u16) {
        self.client.turn_on(slave_id);
    }

    /// Disable the actuators
    ///
    /// Args:
    ///     slave_id (int): The slave id
    pub fn turn_off(&mut self, slave_id: u16) {
        self.client.turn_off(slave_id);
    }

    /// Set the target position
    ///
    /// Args:
    ///     slave_id (int): The slave id
    ///     position (list): The target position
    pub fn set_target_position(&mut self, slave_id: u16, position: Vec<f32>) {
        self.client.set_target_position(slave_id, position);
    }

    /// Set the velocity limit
    ///
    /// Args:
    ///    slave_id (int): The slave id
    ///    velocity_limit (list): Relative velocity limit from 0 to 1
    pub fn set_velocity_limit(&mut self, slave_id: u16, velocity: Vec<f32>) {
        self.client.set_velocity_limit(slave_id, velocity);
    }

    /// Set the torque limit
    ///
    /// Args:
    ///   slave_id (int): The slave id
    ///   torque_limit (list): Relative torque limit from 0 to 1
    pub fn set_torque_limit(&mut self, slave_id: u16, torque: Vec<f32>) {
        self.client.set_torque_limit(slave_id, torque);
    }

    /// Get the actual position
    ///
    /// Args:
    ///     slave_id (int): The slave id
    /// Returns:
    ///     list: The actual position
    pub fn get_position_actual_value(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_position_actual_value(slave_id) {
            Ok(position) => position,
            _ => panic!("Error in getting position actual value"),
        }
    }

    /// Get the target position
    ///
    /// Args:
    ///     slave_id (int): The slave id
    /// Returns:
    ///     list: The target position
    pub fn get_target_position(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_target_position(slave_id) {
            Ok(position) => position,
            _ => panic!("Error in getting target position"),
        }
    }

    /// Get the target velocity
    ///
    /// Args:
    ///     slave_id (int): The slave id
    /// Returns:
    ///     list: The target velocity
    pub fn set_target_velocity(&mut self, slave_id: u16, velocity: Vec<f32>) {
        self.client.set_target_velocity(slave_id, velocity);
    }

    /// Set the target torque
    ///
    /// Args:
    ///     slave_id (int): The slave id
    ///     torque (list): The target torque
    pub fn set_target_torque(&mut self, slave_id: u16, torque: Vec<f32>) {
        self.client.set_target_torque(slave_id, torque);
    }

    /// Get the actual velocity
    ///
    /// Args:
    ///     slave_id (int): The slave id
    /// Returns:
    ///     list: The actual velocity
    pub fn get_velocity_actual_value(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_velocity_actual_value(slave_id) {
            Ok(velocity) => velocity,
            _ => panic!("Error in getting velocity actual value"),
        }
    }

    /// Get the torque velocity
    ///
    /// Args:
    ///     slave_id (int): The slave id
    /// Returns:
    ///     list: The actual torque
    pub fn get_torque_actual_value(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_torque_actual_value(slave_id) {
            Ok(torque) => torque,
            _ => panic!("Error in getting torque actual value"),
        }
    }

    /// Get the current axis sensor values
    ///
    /// Args:
    ///    slave_id (int): The slave id
    /// Returns:
    ///   list: The current axis sensor values
    pub fn get_axis_sensors(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_axis_sensors(slave_id) {
            Ok(sensors) => sensors,
            _ => panic!("Error in getting axis sensors"),
        }
    }

    /// Get the axis sensor zeros in firmware
    ///
    /// Args:
    ///     slave_id (int): The slave id
    /// Returns:
    ///     list: The axis sensor zero values
    pub fn get_axis_sensor_zeros(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_axis_sensor_zeros(slave_id) {
            Ok(zeros) => zeros,
            _ => panic!("Error in getting axis sensor zeros"),
        }
    }

    /// Check if motors are activated
    ///
    /// Args:
    ///    slave_id (int): The slave id
    /// Returns:
    ///     bool: True if the motor is activated, False otherwise
    pub fn get_torque_state(&mut self, slave_id: u16) -> bool {
        match self.client.get_torque_state(slave_id) {
            Ok(state) => state,
            _ => panic!("Error in getting torque state"),
        }
    }

    /// Get the state
    ///
    /// Args:
    ///    slave_id (int): The slave id    
    /// Returns:
    ///     int: The state  (CiA402 state machine)
    pub fn get_state(&mut self, slave_id: u16) -> u32 {
        match self.client.get_state(slave_id) {
            Ok(state) => state,
            _ => panic!("Error in getting state"),
        }
    }

    /// Print the state
    /// Outputs the CiA402 state machine state
    ///
    /// Args:
    ///   slave_id (int): The slave id
    pub fn print_state(&mut self, slave_id: u16) {
        let state = match self.client.get_cia402_state(slave_id) {
            Ok(state) => state,
            _ => panic!("Error in getting state"),
        };

        let cia_state = state_machine::parse_state_from_status_word(state as u16);
        println!("State: {:?}", cia_state);
    }

    /// Get the error codes
    ///
    /// Args:
    ///    slave_id (int): The slave id
    /// Returns:
    ///     list: The error codes  (see poule_ethercat_controller/src/state_machine.rs)
    pub fn get_error_codes(&mut self, slave_id: u16) -> Vec<i32> {
        match self.client.get_error_codes(slave_id) {
            Ok(codes) => codes,
            _ => panic!("Error in getting error codes"),
        }
    }

    /// Print the error codes  
    ///
    /// Args:
    ///   slave_id (int): The slave id
    pub fn print_error_codes(&mut self, slave_id: u16) {
        let error_codes = match self.client.get_error_codes(slave_id) {
            Ok(codes) => codes,
            _ => panic!("Error in getting error codes"),
        };

        // homing error flags
        let homing_error =
            state_machine::parse_homing_error_flags((error_codes[0] as u16).to_le_bytes());
        if homing_error.len() > 0 {
            println!("Homing | error flags: {:?}", homing_error);
        } else {
            println!("Homing | OK!");
        }
        // motor error flags
        for (i, code) in error_codes[1..].iter().enumerate() {
            let m_code = state_machine::parse_motor_error_flags((*code as u16).to_le_bytes());
            if m_code.len() > 0 {
                println!("Motor {} | Error flags: {:?}", i, m_code);
            } else {
                println!("Motor {} | OK!", i);
            }
        }
    }

    /// Get the connected devices
    ///
    /// Returns:
    ///     list(tuple): The connected devices (slave ids, device names)
    pub fn get_connected_devices(&mut self) -> (Vec<u16>, Vec<String>) {
        (self.client.ids.clone(), self.client.names.clone())
    }

    pub fn get_all_slaves_in_network(&mut self) -> (Vec<u16>, Vec<String>) {
        match self.client.get_poulpe_ids_sync() {
            Ok(slaves) => slaves,
            _ => panic!("Error in getting connected devices"),
        }
    }

    /// Get the motor temperatures
    ///
    /// Args:
    ///     slave_id (int): The slave id
    /// Returns:
    ///     list: The motor temperatures
    pub fn get_motor_temperatures(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_motor_temperatures(slave_id) {
            Ok(temps) => temps,
            _ => panic!("Error in getting temperatures"),
        }
    }
    /// Get the board temperatures
    ///
    /// Args:
    ///     slave_id (int): The slave id
    /// Returns:
    ///     list: The board temperatures
    pub fn get_board_temperatures(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_board_temperatures(slave_id) {
            Ok(temps) => temps,
            _ => panic!("Error in getting temperatures"),
        }
    }

    /// Do an emergency stop
    ///
    /// Args:
    ///     slave_id (int): The slave id
    pub fn emergency_stop(&mut self, slave_id: u16) {
        self.client.emergency_stop(slave_id);
    }

    // Define other methods similarly...
}

/// Launch the server
#[pyfunction]
#[pyo3(signature = (file_name=None))]
pub fn launch_server(file_name: Option<&str>) -> String {
    let filename = match file_name {
        Some(name) => name.to_string(),
        None => "../config/ethercat.yaml".to_string(),
    };

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = poulpe_ethercat_grpc::server::launch_server(&filename).await {
                eprintln!("Failed to launch the server: {}", e);
            }
        });
    });
    return "http://127.0.0.1:50098".to_string();
}

/// Launch the server
#[pyfunction]
pub fn get_all_slaves_in_network(addr: &str) -> (Vec<u16>, Vec<String>) {
    let addr_uri = match addr.parse::<Uri>() {
        Ok(uri) => uri,
        Err(_) => panic!("Invalid URI format"),
    };

    match PoulpeIdClient::new(addr_uri).get_slaves() {
        Ok(slaves) => slaves,
        _ => panic!("Error in getting connected devices"),
    }
}

#[pymodule]
fn python_client(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPoulpeRemoteClient>()?;
    // add launch server method
    m.add_function(wrap_pyfunction!(launch_server, m)?)?;
    // add get_all_slaves_in_network method
    m.add_function(wrap_pyfunction!(get_all_slaves_in_network, m)?)?;
    Ok(())
}
