use std::{collections::HashMap, sync::Arc, time::Duration};

use poulpe_ethercat_grpc::client;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use tokio::{
    runtime::{Builder, Runtime},
    sync::RwLock,
    time::sleep,
};
use tonic::{transport::Uri, Request};

use poulpe_ethercat_grpc::client::PoulpeRemoteClient;

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

    pub fn get_mode_of_operation(&mut self, slave_id: u16) -> u32 {
        match self.client.get_mode_of_operation(slave_id) {
            Ok(mode) => mode,
            _ => panic!("Error in getting mode of operation"),
        }
    }
    pub fn set_mode_of_operation(&mut self, slave_id: u16, mode: u32) {
        self.client.set_mode_of_operation(slave_id, mode);
    }

    pub fn turn_on(&mut self, slave_id: u16) {
        self.client.turn_on(slave_id);
    }

    pub fn turn_off(&mut self, slave_id: u16) {
        self.client.turn_off(slave_id);
    }

    pub fn set_target_position(&mut self, slave_id: u16, position: Vec<f32>) {
        self.client.set_target_position(slave_id, position);
    }

    pub fn set_velocity_limit(&mut self, slave_id: u16, velocity: Vec<f32>) {
        self.client.set_velocity_limit(slave_id, velocity);
    }

    pub fn set_torque_limit(&mut self, slave_id: u16, torque: Vec<f32>) {
        self.client.set_torque_limit(slave_id, torque);
    }

    pub fn get_position_actual_value(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_position_actual_value(slave_id) {
            Ok(position) => position,
            _ => panic!("Error in getting position actual value"),
        }
    }

    pub fn get_target_position(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_target_position(slave_id) {
            Ok(position) => position,
            _ => panic!("Error in getting target position"),
        }
    }

    pub fn set_target_velocity(&mut self, slave_id: u16, velocity: Vec<f32>) {
        self.client.set_target_velocity(slave_id, velocity);
    }
    pub fn set_target_torque(&mut self, slave_id: u16, torque: Vec<f32>) {
        self.client.set_target_torque(slave_id, torque);
    }

    pub fn get_velocity_actual_value(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_velocity_actual_value(slave_id) {
            Ok(velocity) => velocity,
            _ => panic!("Error in getting velocity actual value"),
        }
    }

    pub fn get_torque_actual_value(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_torque_actual_value(slave_id) {
            Ok(torque) => torque,
            _ => panic!("Error in getting torque actual value"),
        }
    }

    pub fn get_axis_sensors(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_axis_sensors(slave_id) {
            Ok(sensors) => sensors,
            _ => panic!("Error in getting axis sensors"),
        }
    }

    pub fn get_axis_sensor_zeros(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_axis_sensor_zeros(slave_id) {
            Ok(zeros) => zeros,
            _ => panic!("Error in getting axis sensor zeros"),
        }
    }

    pub fn get_torque_state(&mut self, slave_id: u16) -> bool {
        match self.client.get_torque_state(slave_id) {
            Ok(state) => state,
            _ => panic!("Error in getting torque state"),
        }
    }

    pub fn get_state(&mut self, slave_id: u16) -> u32 {
        match self.client.get_state(slave_id) {
            Ok(state) => state,
            _ => panic!("Error in getting state"),
        }
    }

    pub fn get_error_codes(&mut self, slave_id: u16) -> Vec<i32> {
        match self.client.get_error_codes(slave_id) {
            Ok(codes) => codes,
            _ => panic!("Error in getting error codes"),
        }
    }

    pub fn get_connected_devices(&mut self) -> (Vec<u16>, Vec<String>) {
        match self.client.get_poulpe_ids_sync() {
            Ok(ids) => ids,
            _ => panic!("Error in getting connected devices"),
        }
    }

    pub fn get_motor_temperatures(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_motor_temperatures(slave_id) {
            Ok(temps) => temps,
            _ => panic!("Error in getting temperatures"),
        }
    }
    pub fn get_board_temperatures(&mut self, slave_id: u16) -> Vec<f32> {
        match self.client.get_board_temperatures(slave_id) {
            Ok(temps) => temps,
            _ => panic!("Error in getting temperatures"),
        }
    }

    pub fn emergency_stop(&mut self, slave_id: u16) {
        self.client.emergency_stop(slave_id);
    }

    // Define other methods similarly...
}

#[pymodule]
fn python_client(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPoulpeRemoteClient>()?;
    Ok(())
}
