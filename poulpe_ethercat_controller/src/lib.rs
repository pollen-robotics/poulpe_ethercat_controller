use std::{collections::HashMap, error::Error, f32::consts::PI, sync::RwLock, time::Duration};

extern crate num;
#[macro_use]
extern crate num_derive;

use bitvec::prelude::*;

use ethercat_controller::{
    config::{PoulpeKind, SlaveConfig},
    Config, EtherCatController,
};

mod register;
use register::{BoardStatus, PdoRegister};

#[derive(Debug)]
pub struct PoulpeController {
    inner: EtherCatController,
    poulpe_config: HashMap<u16, PoulpeKind>,
}

impl PoulpeController {
    pub fn connect(filename: &str) -> Result<Self, Box<dyn Error>> {
        let config = Config::from_yaml(filename)?;

        let controller = EtherCatController::open(
            config.ethercat.master_id,
            Duration::from_millis(2),
        )?
        .wait_for_ready();

        let mut poulpe_config = HashMap::new();

        for slave in config.slaves {
            if let SlaveConfig::Poulpe(poulpe) = slave {
                poulpe_config.insert(poulpe.id, poulpe);
            }
        }

        // check if all slaves are connected
        // check if the names of the slaves are correct
        let slave_ids = controller.get_slave_ids();
        for slave_id in slave_ids {
            if poulpe_config.get(&slave_id).is_none() {
                log::error!("Slave {} with name {:?} not found in config, check config yaml file!", slave_id, controller.get_slave_name(slave_id).unwrap());
                return Err("Slave not in yaml!".into());
            }
            else if let Some(name) = controller.get_slave_name(slave_id) {
                if poulpe_config[&slave_id].name != name {
                    log::error!("Slave {} Name mismatch: expected {:?}, got {:?}", slave_id, poulpe_config[&slave_id].name, name);
                    return Err("Name mismatch".into());
                }
                } else {
                    log::error!("Slave {} with name {:?} name not found, check connection!", slave_id, poulpe_config[&slave_id].name);
                    return Err("Name not found, check connection!".into());
                }
        }

        //check if the number of slaves is correct
        let slave_ids = controller.get_slave_ids();
        let mut all_connected = true;
        if slave_ids.len() != poulpe_config.len() {
            for p in poulpe_config.keys(){
                let name = controller.get_slave_name(*p);
                if name.is_none() {
                    log::error!("Slave {} with name {:?} not found in Ethercat network, check connection!", p, poulpe_config[p].name);
                    all_connected = false;
                }
            }
        }
        if all_connected == false{
            return Err("Number of slaves in config and in network do not match!".into());
        }
        
        Ok(Self {
            inner: controller,
            poulpe_config,
        })
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
        self.poulpe_config.values().map(|x| x.name.clone()).collect()
    }




    // fn wait_for_status_bit(&self, slave_id: u16, satus: BoardStatus) {
    //     let status_word = self.get_status(slave_id);
    //     log::info!("Waiting for {:?} ({:?})", bit, satus);

    //     loop {
    //         let status_word = self.get_status(slave_id);

    //         if status_word.contains(&StatusBit::Fault) {
    //             log::error!(
    //                 "Fault status {:#x?} on slave {:?}",
    //                 self.get_error_code(slave_id),
    //                 slave_id
    //             );
    //             self.clear_fault(slave_id);
    //             self.inner.wait_for_next_cycle();
    //             continue;
    //         }

    //         if status_word.contains(&bit) {
    //             break;
    //         }
    //         self.inner.wait_for_next_cycle();
    //     }
    // }

    fn get_pdo_register(&self, slave_id: u16, reg: PdoRegister, index: usize) -> Vec<u8> {
        self.inner
            .get_pdo_register(slave_id, &reg.name().to_string(), index)
            .unwrap()
    }
    fn set_pdo_register(&self, slave_id: u16, reg: PdoRegister, index: usize, value: &[u8]) {
        self.inner
            .set_pdo_register(slave_id, &reg.name().to_string(), index, value.to_vec())
    }

    fn get_pdo_registers(&self, slave_id: u16, reg: PdoRegister) -> Vec<Vec<u8>> {
        self.inner
            .get_pdo_registers(slave_id, &reg.name().to_string())
            .unwrap()
    }
    fn set_pdo_registers(&self, slave_id: u16, reg: PdoRegister, values: Vec<Vec<u8>>) {
        self.inner
            .set_pdo_registers(slave_id, &reg.name().to_string(), values);
    }
}

impl PoulpeController {
    pub fn setup(&self, id: u32) -> Result<(), Box<dyn Error>> {
        let slave_id = id as u16;

        // check if slave_id exists in etheract network
        if !self.inner.get_slave_ids().contains(&slave_id) {
            log::error!("Slave {} with name {:?} not found in Ethercat network, check connection!", id, self.poulpe_config[&slave_id].name);
            return Err("Slave not connected!".into());
        }
        
        // check if slave_id exists in config
        if !self.get_slave_ids().contains(&id) {
            log::error!("Slave {} with name {:?} not found in config, check config yaml file!", id, self.get_slave_name(slave_id).unwrap());
            return Err("Slave not in yaml!".into());
        }

        match self.inner.get_slave_name(slave_id){
            Some(name) => {
                if self.poulpe_config[&slave_id].name != name{
                    log::error!("Slave {} Name mismatch: expected {:?}, got {:?}", slave_id, self.poulpe_config[&slave_id].name, name);
                    return Err("Name mismatch".into());
                }
            },
            _ => {
                log::error!("Slave {} name not found, check connection!", slave_id);
                return Err("Name not found, check connection!".into());
            }
        }

        let no_axes = self.poulpe_config[&slave_id].orbita_type;
        let orbita_type = self.get_pdo_register(slave_id, PdoRegister::OrbitaType, 0)[0] as u32;

        if orbita_type != no_axes {
            log::error!(
                "Slave {} Orbita type mismatch: expected {}, got {}",
                slave_id,
                no_axes,
                orbita_type
            );
            return Err("Orbita type mismatch".into());
        }


        log::info!("Setup of slave {} done!", id);

        Ok(())
    }

    pub fn is_torque_on(&self, id: u32) -> Result<Option<bool>, Box<dyn std::error::Error>> {
        let slave_id = id as u16;
        let status = self.get_pdo_register(slave_id, PdoRegister::TroqueOn, 0)[0];
        let no_motors = self.poulpe_config[&slave_id].orbita_type;
        let mut torque_on = true;
        log::debug!(
            "Checking torque on slave {} with status {}, {}",
            id,
            status,
            no_motors
        );
        for i in 0..no_motors {
            if status & (1 << i) == 0 {
                torque_on = false;
                break;
            }
        }
        Ok(Some(torque_on))
    }
    pub fn set_torque(
        &self,
        id: u32,
        requested_torque: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let slave_id = id as u16;
        if let Some(actual_torque) = self.is_torque_on(id)? {
            log::debug!(
                "Setting torque on slave {} to {} from {}",
                id,
                requested_torque,
                actual_torque
            );
            if actual_torque == requested_torque {
                return Ok(());
            } else {
                let no_motors = self.poulpe_config[&slave_id].orbita_type;
                let mut torque_on: u8 = 0x0;
                if requested_torque {
                    for i in 0..no_motors {
                        torque_on |= 1 << i;
                    }
                }
                self.set_pdo_register(slave_id, PdoRegister::TroqueState, 0, &[torque_on]);
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

    pub fn get_current_temperature(
        &self,
        _id: u32,
    ) -> Result<Option<f32>, Box<dyn std::error::Error>> {
        Ok(Some(42.0))
    }

    pub fn get_status(&self, slave_id: u32) -> BoardStatus {
        let byte = self.get_pdo_register(slave_id as u16, PdoRegister::State, 0)[0];
        BoardStatus::from_u8(byte)
    }

    pub fn get_type(&self, slave_id: u32) -> u8 {
        let byte = self.get_pdo_register(slave_id as u16, PdoRegister::OrbitaType, 0)[0];
        byte
    }

    fn get_register_values(
        &self,
        id: u32,
        register: PdoRegister,
    ) -> Result<Option<Vec<f32>>, Box<dyn std::error::Error>> {
        let slave_id = id as u16;
        let bytes = self.get_pdo_registers(slave_id, register);
        let values = bytes
            .iter()
            .map(|x| f32::from_le_bytes(x[0..4].try_into().unwrap()))
            .collect::<Vec<f32>>();
        Ok(Some(values))
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

    fn set_register_values(
        &self,
        id: u32,
        register: PdoRegister,
        values: Vec<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let slave_id = id as u16;
        let values_bytes: Vec<Vec<u8>> = values.iter().map(|&x| x.to_le_bytes().to_vec()).collect();
        self.set_pdo_registers(slave_id, register, values_bytes);
        Ok(())
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

}
