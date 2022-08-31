use std::{
    env,
    f32::consts::PI,
    io, thread,
    time::{Duration, SystemTime},
};

use ethercat_controller::EtherCatController;

pub enum Slave {
    Id0 = 0,
    Id1 = 1,
    Id2 = 2,
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
            PdoRegister::ControlWord => todo!(),
            PdoRegister::ModeOfOperation => todo!(),
            PdoRegister::TargetPosition => todo!(),
            PdoRegister::VelocityOffset => todo!(),
            PdoRegister::TargetTorque => todo!(),
            PdoRegister::StatusWord => todo!(),
            PdoRegister::ModeOfOperationDisplay => todo!(),
            PdoRegister::PositionActualValue => todo!(),
            PdoRegister::VelocityActualValue => todo!(),
            PdoRegister::TorqueActualValue => todo!(),
            PdoRegister::ErrorCode => todo!(),
        }
    }
    fn length(&self) -> usize {
        match *self {
            PdoRegister::ControlWord => todo!(),
            PdoRegister::ModeOfOperation => todo!(),
            PdoRegister::TargetPosition => todo!(),
            PdoRegister::VelocityOffset => todo!(),
            PdoRegister::TargetTorque => todo!(),
            PdoRegister::StatusWord => todo!(),
            PdoRegister::ModeOfOperationDisplay => todo!(),
            PdoRegister::PositionActualValue => todo!(),
            PdoRegister::VelocityActualValue => todo!(),
            PdoRegister::TorqueActualValue => todo!(),
            PdoRegister::ErrorCode => todo!(),
        }
    }
}

struct EposController {
    controller: EtherCatController,
}

impl EposController {
    fn new(filename: &String, master_id: u32) -> Result<Self, io::Error> {
        Ok(Self {
            controller: EtherCatController::open(filename, master_id)?,
        })
    }

    fn get_controlworld(&self) -> u16 {
        let bytes = self.get_pdo_register(PdoRegister::ControlWord);
        u16::from_le_bytes(bytes.try_into().unwrap())
    }

    fn set_controlword(&self, value: u16) {
        self.set_pdo_register(PdoRegister::ControlWord, &value.to_le_bytes())
    }

    fn get_mode_of_operation(&self) -> u8 {
        self.get_pdo_register(PdoRegister::ModeOfOperation)[0]
    }

    fn set_mode_of_operation(&self, value: u8) {
        self.set_pdo_register(PdoRegister::ModeOfOperation, &[value])
    }

    fn get_target_position(&self) -> u32 {
        let bytes = self.get_pdo_register(PdoRegister::TargetPosition);
        u32::from_le_bytes(bytes.try_into().unwrap())
    }

    fn set_target_position(&self, value: u32) {
        self.set_pdo_register(PdoRegister::TargetPosition, &value.to_le_bytes())
    }

    fn get_position_actual_value(&self) -> u32 {
        let bytes = self.get_pdo_register(PdoRegister::PositionActualValue);
        u32::from_le_bytes(bytes.try_into().unwrap())
    }

    fn get_pdo_register(&self, reg: PdoRegister) -> Vec<u8> {
        self.controller
            .get_pdo_register(reg.addr(), reg.length())
            .unwrap()
    }

    fn set_pdo_register(&self, reg: PdoRegister, value: &[u8]) {
        self.controller
            .set_pdo_register(reg.addr(), reg.length(), value.to_vec())
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

    // Setup Modes of operation to Profile Position Mode
    epos_controller.set_mode_of_operation(0x01);
    thread::sleep(Duration::from_millis(1));

    // Shutdown
    epos_controller.set_controlword(0x06);
    thread::sleep(Duration::from_millis(1));

    // Switch On
    epos_controller.set_controlword(0x07);
    thread::sleep(Duration::from_millis(1));

    // Switch On & Enable
    epos_controller.set_controlword(0x0F);
    thread::sleep(Duration::from_millis(1));

    let f = 0.5_f32;
    let amp = 2000.0_f32;

    let t = SystemTime::now();

    loop {
        let pos = epos_controller.get_position_actual_value();
        println!("Pos: {}", pos);

        let dt = t.elapsed().unwrap().as_secs_f32();
        let target_pos = amp * (2.0 * PI * f * dt).sin();
        let target_pos: u32 = (target_pos as i32 + 5000) as u32;

        // Set controlword to (Absolute pos, start immediatly)
        epos_controller.set_controlword(0x3F);
        epos_controller.set_target_position(target_pos);

        epos_controller.wait_for_next_cycle();
        // Set controlword (Absolute pos, start immediatly)
        epos_controller.set_controlword(0x0F);

        thread::sleep(Duration::from_millis(10));
    }
}
