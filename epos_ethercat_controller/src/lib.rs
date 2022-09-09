extern crate num;
#[macro_use]
extern crate num_derive;

use std::{io, time::Duration, convert::TryInto};

use bitvec::prelude::*;

use ethercat_controller::EtherCatController;

enum PdoRegister {
    ControlWord,
    ModeOfOperation,
    TargetPosition,
    #[allow(dead_code)]
    VelocityOffset,
    #[allow(dead_code)]
    TargetTorque,

    StatusWord,
    ModeOfOperationDisplay,
    PositionActualValue,
    VelocityActualValue,
    TorqueActualValue,
    ErrorCode,
    HallSensor,
    AbsoluteEncoder,
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
            PdoRegister::HallSensor => "Hall sensor",
            PdoRegister::AbsoluteEncoder => "Absolute encoder",
        }
    }
}

#[derive(Debug)]
pub struct EposController {
    controller: EtherCatController,
}

#[derive(FromPrimitive, Debug, PartialEq)]
enum StatusBit {
    ReadyToSwitchOn = 0,
    SwitchedOn = 1,
    OperationEnabled = 2,
    Fault = 3,
    VoltageEnabled = 4,
    QuickStop = 5,
    SwitchedOnDisabled = 6,
    Warning = 7,
    Reserved8 = 8,
    Remote = 9,
    OperatingModeSpecific10 = 10,
    InternalLimitActive = 11,
    OperatingModeSpecific12 = 12,
    OperatingModeSpecific13 = 13,
    Reserved14 = 14,
    PositionReferencedToHomePosition = 15,
}

impl EposController {
    pub fn connect(filename: &String, master_id: u32) -> Result<Self, io::Error> {
        let controller = EtherCatController::open(filename, master_id, Duration::from_millis(1))?
            .wait_for_ready();

        Ok(Self { controller })
    }

    pub fn get_slave_ids(&self) -> Vec<u16> {
        self.controller.get_slave_ids()
    }

    pub fn setup(&self, slave_id: u16) {
        self.wait_for_status_bit(slave_id, StatusBit::SwitchedOnDisabled);

        // Setup Modes of operation to Cyclic Synchronous Position Mode
        self.set_mode_of_operation(slave_id, 0x08, true);
    }

    pub fn turn_on(&self, slave_id: u16, reset_target: bool) {
        if reset_target {
            // Set Target Position to Actual Pos
            let actual_pos = self.get_position_actual_value(slave_id);
            self.set_target_position(slave_id, actual_pos);
        }

        // Shutdown
        self.set_controlword(slave_id, 0x06);
        self.wait_for_status_bit(slave_id, StatusBit::ReadyToSwitchOn);

        // Switch On & Enable
        self.set_controlword(slave_id, 0x0F);
        self.wait_for_status_bit(slave_id, StatusBit::SwitchedOn);
        self.wait_for_status_bit(slave_id, StatusBit::OperationEnabled);
        self.wait_for_status_bit(slave_id, StatusBit::VoltageEnabled);
    }

    pub fn turn_off(&self, slave_id: u16) {
        // Shutdown
        self.set_controlword(slave_id, 0x06);
        self.wait_for_status_bit(slave_id, StatusBit::ReadyToSwitchOn);
    }

    pub fn is_on(&self, slave_id: u16) -> bool {
        let status = self.get_status_word(slave_id);
        status.contains(&StatusBit::SwitchedOn)
    }

    #[allow(dead_code)]
    fn get_controlworld(&self, slave_id: u16) -> u16 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::ControlWord);
        u16::from_le_bytes(bytes.try_into().unwrap())
    }

    fn set_controlword(&self, slave_id: u16, value: u16) {
        self.set_pdo_register(slave_id, PdoRegister::ControlWord, &value.to_le_bytes())
    }

    fn get_mode_of_operation(&self, slave_id: u16) -> u8 {
        self.get_pdo_register(slave_id, PdoRegister::ModeOfOperation)[0]
    }

    fn set_mode_of_operation(&self, slave_id: u16, value: u8, wait: bool) {
        self.set_pdo_register(slave_id, PdoRegister::ModeOfOperation, &[value]);

        if wait {
            loop {
                let mode = self.get_mode_of_operation(slave_id);
                let mode_display = self.get_mode_of_operation_display(slave_id);

                if mode == value && mode_display == value {
                    break;
                }

                self.wait_for_next_cycle();
            }
        }
    }

    pub fn get_target_position(&self, slave_id: u16) -> i32 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::TargetPosition);
        i32::from_le_bytes(bytes.try_into().unwrap())
    }

    pub fn set_target_position(&self, slave_id: u16, value: i32) {
        self.set_pdo_register(slave_id, PdoRegister::TargetPosition, &value.to_le_bytes())
    }

    #[allow(dead_code)]
    fn get_velocity_offset(&self, slave_id: u16) -> i32 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::VelocityOffset);
        i32::from_le_bytes(bytes.try_into().unwrap())
    }

    #[allow(dead_code)]
    fn set_velocity_offset(&self, slave_id: u16, value: i32) {
        self.set_pdo_register(slave_id, PdoRegister::VelocityOffset, &value.to_le_bytes())
    }

    #[allow(dead_code)]
    fn get_target_torque(&self, slave_id: u16) -> i16 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::TargetTorque);
        i16::from_le_bytes(bytes.try_into().unwrap())
    }

    #[allow(dead_code)]
    fn set_target_torque(&self, slave_id: u16, value: i16) {
        self.set_pdo_register(slave_id, PdoRegister::TargetTorque, &value.to_le_bytes())
    }

    fn get_status_word(&self, slave_id: u16) -> Vec<StatusBit> {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::StatusWord);

        let bits = u16::from_le_bytes(bytes.try_into().unwrap());
        let bits = bits.view_bits::<Lsb0>();

        (0..16)
            .filter(|i| *bits.get(*i).unwrap())
            .map(|b| -> StatusBit { num::FromPrimitive::from_usize(b).unwrap() })
            .collect()
    }

    fn get_mode_of_operation_display(&self, slave_id: u16) -> u8 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::ModeOfOperationDisplay);
        u8::from_le_bytes(bytes.try_into().unwrap())
    }

    pub fn get_position_actual_value(&self, slave_id: u16) -> i32 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::PositionActualValue);
        i32::from_le_bytes(bytes.try_into().unwrap())
    }

    pub fn get_velocity_actual_value(&self, slave_id: u16) -> i32 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::VelocityActualValue);
        i32::from_le_bytes(bytes.try_into().unwrap())
    }

    pub fn get_torque_actual_value(&self, slave_id: u16) -> i16 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::TorqueActualValue);
        i16::from_le_bytes(bytes.try_into().unwrap())
    }

    pub fn get_error_code(&self, slave_id: u16) -> u16 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::ErrorCode);
        u16::from_le_bytes(bytes.try_into().unwrap())
    }

    pub fn get_hall_sensor_value(&self, slave_id: u16) -> i32 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::HallSensor);
        i32::from_le_bytes(bytes.try_into().unwrap())
    }

    pub fn get_absolute_encoder_value(&self, slave_id: u16) -> i32 {
        let bytes = self.get_pdo_register(slave_id, PdoRegister::AbsoluteEncoder);
        i32::from_le_bytes(bytes.try_into().unwrap())
    }

    fn get_pdo_register(&self, slave_id: u16, reg: PdoRegister) -> Vec<u8> {
        self.controller
            .get_pdo_register(slave_id, &reg.name().to_string())
            .unwrap()
    }

    fn set_pdo_register(&self, slave_id: u16, reg: PdoRegister, value: &[u8]) {
        self.controller
            .set_pdo_register(slave_id, &reg.name().to_string(), value.to_vec())
    }

    pub fn wait_for_next_cycle(&self) {
        self.controller.wait_for_next_cycle()
    }

    fn wait_for_status_bit(&self, slave_id: u16, bit: StatusBit) {
        loop {
            if self.get_status_word(slave_id).contains(&bit) {
                break;
            }
            self.wait_for_next_cycle();
        }
    }
}
