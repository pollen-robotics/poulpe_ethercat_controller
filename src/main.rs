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
    fn addr(&self) -> usize {
        match *self {
            PdoRegister::ControlWord => 0,
            PdoRegister::ModeOfOperation => 2,
            PdoRegister::TargetPosition => 3,
            PdoRegister::VelocityOffset => 7,
            PdoRegister::TargetTorque => 11,
            PdoRegister::StatusWord => 13,
            PdoRegister::ModeOfOperationDisplay => 15,
            PdoRegister::PositionActualValue => 16,
            PdoRegister::VelocityActualValue => 20,
            PdoRegister::TorqueActualValue => 24,
            PdoRegister::ErrorCode => 26,
        }
    }
    fn length(&self) -> usize {
        match *self {
            PdoRegister::ControlWord => 2,
            PdoRegister::ModeOfOperation => 1,
            PdoRegister::TargetPosition => 4,
            PdoRegister::VelocityOffset => 4,
            PdoRegister::TargetTorque => 2,
            PdoRegister::StatusWord => 2,
            PdoRegister::ModeOfOperationDisplay => 1,
            PdoRegister::PositionActualValue => 4,
            PdoRegister::VelocityActualValue => 4,
            PdoRegister::TorqueActualValue => 2,
            PdoRegister::ErrorCode => 2,
        }
    }
}

struct EposController {
    controller: EtherCatController,
    offset: usize,
}

impl EposController {
    fn new(filename: &String, master_id: u32) -> Result<Self, io::Error> {
        Ok(Self {
            controller: EtherCatController::open(filename, master_id, Duration::from_millis(1))?,
            offset: 28,
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

    fn get_position_actual_value(&self, slave_id: &Slave) -> u32 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::PositionActualValue);
        u32::from_le_bytes(bytes.try_into().unwrap())
    }

    fn get_pdo_register(&self, slave_id: &Slave, reg: PdoRegister) -> Vec<u8> {
        let offset = slave_id.offset() * self.offset;

        self.controller
            .get_pdo_register(offset + reg.addr(), reg.length())
            .unwrap()
    }

    fn set_pdo_register(&self, slave_id: &Slave, reg: PdoRegister, value: &[u8]) {
        let offset = slave_id.offset() * self.offset;

        self.controller
            .set_pdo_register(offset + reg.addr(), reg.length(), value.to_vec())
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

    let slaves = vec![Slave::Id0, Slave::Id1, Slave::Id2];

    // Wait for device initialisation
    thread::sleep(Duration::from_secs(2));

    // Setup Modes of operation to Profile Position Mode
    for slave in slaves.iter() {
        epos_controller.set_mode_of_operation(&slave, 0x01);
    }
    thread::sleep(Duration::from_millis(10));

    // Shutdown
    for slave in slaves.iter() {
        epos_controller.set_controlword(&slave, 0x06);
    }
    thread::sleep(Duration::from_millis(10));

    // Switch On
    for slave in slaves.iter() {
        epos_controller.set_controlword(&slave, 0x07);
    }
    thread::sleep(Duration::from_millis(10));

    // Switch On & Enable
    for slave in slaves.iter() {
        epos_controller.set_controlword(&slave, 0x0F);
    }
    thread::sleep(Duration::from_millis(10));

    let f = 0.5_f32;
    let amp = 2000.0_f32;

    let t = SystemTime::now();

    loop {
        // let pos = epos_controller.get_position_actual_value(slave);

        let dt = t.elapsed().unwrap().as_secs_f32();
        let target_pos = amp * (2.0 * PI * f * dt).sin();
        let target_pos: u32 = (target_pos as i32 + 5000) as u32;

        // println!("Target: {} Current: {} Error: {}", target_pos, pos, pos as i32 - target_pos as i32);

        // Set controlword to (Absolute pos, start immediatly)
        for slave in slaves.iter() {
            epos_controller.set_controlword(&slave, 0x3F);
            epos_controller.set_target_position(&slave, target_pos);
        }

        // WTF...
        thread::sleep(Duration::from_millis(10));

        // Set controlword (Absolute pos, start immediatly)
        for slave in slaves.iter() {
            epos_controller.set_controlword(&slave, 0x0F);
        }

        thread::sleep(Duration::from_millis(10));
    }
}
