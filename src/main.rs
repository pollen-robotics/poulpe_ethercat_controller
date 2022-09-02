use std::{
    env,
    f32::consts::PI,
    io, thread,
    time::{Duration, SystemTime},
};

use ethercat_controller::EtherCatController;

pub enum Slave {
    Id0,
    Id1,
    Id2,
}

impl Slave {
    fn offset(&self) -> usize {
        match *self {
            Slave::Id0 => 0,
            Slave::Id1 => 1,
            Slave::Id2 => 2,
        }
    }
}


enum PdoRegister {
    ControlWord,
    ModeOfOperation,
    TargetPosition,
    VelocityOffset,
    TargetTorque,

    StatusWord,
    ModeOfOperationDisplay,
    PositionActualValue,
    VelocityActualValue,
    TorqueActualValue,
    ErrorCode,
}

impl PdoRegister {
    fn name(&self) -> &'static str {
        match *self {
            PdoRegister::ControlWord => "Controlword",
            PdoRegister::ModeOfOperation => "Mode of operation",
            PdoRegister::TargetPosition => "Target position",
            PdoRegister::VelocityOffset => "Velocity offset",
            PdoRegister::TargetTorque => "Target torque",
            PdoRegister::StatusWord => "Statusword",
            PdoRegister::ModeOfOperationDisplay => "Mode of operation display",
            PdoRegister::PositionActualValue => "Position actual value",
            PdoRegister::VelocityActualValue => "Velocity actual value",
            PdoRegister::TorqueActualValue => "Torque actual value",
            PdoRegister::ErrorCode => "Error code",
        }
    }
}

struct EposController {
    controller: EtherCatController,
}

impl EposController {
    fn new(filename: &String, master_id: u32) -> Result<Self, io::Error> {
        Ok(Self {
            controller: EtherCatController::open(filename, master_id, Duration::from_millis(1))?,
        })
    }

    fn get_controlworld(&self, slave_id: &Slave) -> u16 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::ControlWord);
        u16::from_le_bytes(bytes.try_into().unwrap())
    }

    fn set_controlword(&self, slave_id: &Slave, value: u16) {
        self.set_pdo_register(slave_id, PdoRegister::ControlWord, &value.to_le_bytes())
    }

    fn get_mode_of_operation(&self, slave_id: &Slave) -> u8 {
        self.get_pdo_register(slave_id, PdoRegister::ModeOfOperation)[0]
    }

    fn set_mode_of_operation(&self, slave_id: &Slave, value: u8) {
        self.set_pdo_register(slave_id, PdoRegister::ModeOfOperation, &[value])
    }

    fn get_target_position(&self, slave_id: &Slave) -> u32 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::TargetPosition);
        u32::from_le_bytes(bytes.try_into().unwrap())
    }

    fn set_target_position(&self, slave_id: &Slave, value: u32) {
        self.set_pdo_register(slave_id, PdoRegister::TargetPosition, &value.to_le_bytes())
    }

    fn get_velocity_offset(&self, slave_id: &Slave) -> u32 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::VelocityOffset);
        u32::from_le_bytes(bytes.try_into().unwrap())
    }

    fn set_velocity_offset(&self, slave_id: &Slave, value: u32) {
        self.set_pdo_register(slave_id, PdoRegister::VelocityOffset, &value.to_le_bytes())
    }

    fn get_position_actual_value(&self, slave_id: &Slave) -> u32 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::PositionActualValue);
        u32::from_le_bytes(bytes.try_into().unwrap())
    }

    fn get_pdo_register(&self, slave_id: &Slave, reg: PdoRegister) -> Vec<u8> {
        self.controller
            .get_pdo_register(slave_id.offset() as u16, &reg.name().to_string())
            .unwrap()
    }

    fn set_pdo_register(&self, slave_id: &Slave, reg: PdoRegister, value: &[u8]) {
        self.controller
            .set_pdo_register(slave_id.offset() as u16, &reg.name().to_string(), value.to_vec())
    }

    fn wait_for_next_cycle(&self) {
        self.controller.wait_for_next_cycle()
    }
}

fn main() -> Result<(), io::Error> {
    env_logger::init();

    let args: Vec<_> = env::args().collect();
    let filename = match args.len() {
        2 => &args[1],
        _ => {
            println!("usage: {} ESI-FILE", env!("CARGO_PKG_NAME"));
            return Ok(());
        }
    };

    let epos_controller = EposController::new(filename, 0_u32)?;

    // Wait for device initialisation
    thread::sleep(Duration::from_secs(2));

    // Setup Modes of operation to Cyclic Synchronous Position Mode
    epos_controller.set_mode_of_operation(&Slave::Id1, 0x08);
    epos_controller.set_mode_of_operation(&Slave::Id2, 0x08);
    thread::sleep(Duration::from_millis(10));

    // Shutdown
    epos_controller.set_controlword(&Slave::Id1, 0x06);
    epos_controller.set_controlword(&Slave::Id2, 0x06);
    
    thread::sleep(Duration::from_millis(10));

    // Switch On & Enable
    epos_controller.set_controlword(&Slave::Id1, 0x0F);
    epos_controller.set_controlword(&Slave::Id2, 0x0F);
    thread::sleep(Duration::from_millis(10));

    loop {
        let pos = epos_controller.get_position_actual_value(&Slave::Id0);

        epos_controller.set_target_position(&Slave::Id1, pos);
        epos_controller.set_target_position(&Slave::Id2, pos);

        epos_controller.wait_for_next_cycle();
    }
}
