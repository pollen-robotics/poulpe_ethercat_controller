use crate::state_machine::{CiA402State, ErrorFlags, HomingErrorFlag, MotorErrorFlag};

pub(crate) enum PdoRegister {
    ErrorCode,
    ActuatorType,
    AxisZeroPosition,
    BoardTemperature,
    MotorTemperature,

    ControlWord,
    ModeOfOperation,
    TargetPosition,
    TargetVelocity,
    TargetTorque,
    VelocityLimit,
    TorqueLimit,

    StatusWord,
    ModeOfOperationDisplay,
    PositionActualValue,
    VelocityActualValue,
    TorqueActualValue,
    AxisSensorActualValue,
}

impl PdoRegister {
    pub(crate) fn name(&self) -> &'static str {
        match *self {
            PdoRegister::ErrorCode => "error_code",
            PdoRegister::ActuatorType => "actuator_type",
            PdoRegister::AxisZeroPosition => "axis_position_zero_offset",
            PdoRegister::BoardTemperature => "board_temperatures",
            PdoRegister::MotorTemperature => "motor_temperatures",

            PdoRegister::ControlWord => "controlword",
            PdoRegister::ModeOfOperation => "mode_of_operation",
            PdoRegister::TargetTorque => "target_torque",
            PdoRegister::TargetPosition => "target_position",
            PdoRegister::TargetVelocity => "target_velocity",
            PdoRegister::VelocityLimit => "velocity_limit",
            PdoRegister::TorqueLimit => "torque_limit",

            PdoRegister::StatusWord => "statusword",
            PdoRegister::ModeOfOperationDisplay => "mode_of_operation_display",
            PdoRegister::PositionActualValue => "actual_position",
            PdoRegister::VelocityActualValue => "actual_velocity",
            PdoRegister::TorqueActualValue => "actual_torque",
            PdoRegister::AxisSensorActualValue => "actual_axis_position",
        }
    }
}

// taken from  https://github.com/pollen-robotics/firmware_Poulpe/blob/dbafefe93868296ace1517908a2280c336af03a4/src/motor_control/mod.rs#L20-L31
#[derive(FromPrimitive, Debug, PartialEq)]
#[repr(u8)]
pub enum BoardStatus {
    Ok = 0,
    InitError = 1,
    SensorError = 2,
    IndexError = 3,
    ZeroingError = 4,
    OverTemperatureError = 5,
    OverCurrentError = 6,
    BusVoltageError = 7,
    Init = 20,
    HighTemperatureState = 100,
    Unknown = 255,
}

impl BoardStatus {
    pub fn from_cia402_to_board_status(state: u32, flags: Vec<i32>) -> Result<BoardStatus, ()> {
        let state: CiA402State = num::FromPrimitive::from_u32(state).unwrap();

        let homing_errors: Vec<HomingErrorFlag> = flags
            .first()
            .map(|x| {
                let error = u16::from_le_bytes((*x as u16).to_le_bytes());
                let mut error_flags = Vec::new();
                for i in 0..16 {
                    if error & (1 << i) != 0 {
                        error_flags.push(num::FromPrimitive::from_u16(i as u16).unwrap());
                    }
                }
                error_flags
            })
            .unwrap_or(Vec::new());

        // start from the second flags element
        let motor_errors: Vec<Vec<MotorErrorFlag>> = flags[1..]
            .iter()
            .map(|x| {
                let error = u16::from_le_bytes((*x as u16).to_le_bytes());
                let mut error_flags = Vec::new();
                for i in 0..16 {
                    if error & (1 << i) != 0 {
                        error_flags.push(num::FromPrimitive::from_u16(i as u16).unwrap());
                    }
                }
                error_flags
            })
            .collect();

        let flags = ErrorFlags {
            motor_error_flags: motor_errors,
            homing_error_flags: homing_errors,
        };

        // baordstatus is much less informative than the error flags so we are obliged to make some strange decisions here
        match state {
            CiA402State::NotReadyToSwitchOn => Ok(BoardStatus::Init),
            CiA402State::SwitchOnDisabled
            | CiA402State::ReadyToSwitchOn
            | CiA402State::SwitchedOn
            | CiA402State::OperationEnabled
            | CiA402State::QuickStopActive => {
                if flags
                    .motor_error_flags
                    .iter()
                    .any(|x| x.contains(&MotorErrorFlag::HighTemperatureWarning))
                {
                    Ok(BoardStatus::HighTemperatureState)
                } else {
                    Ok(BoardStatus::Ok)
                }
            }
            CiA402State::Fault | CiA402State::FaultReactionActive => {
                if flags.motor_error_flags.iter().any(|x| {
                    x.contains(&MotorErrorFlag::OverTemperatureMotor)
                        || x.contains(&MotorErrorFlag::OverTemperatureBoard)
                }) {
                    Ok(BoardStatus::OverTemperatureError)
                } else if flags
                    .motor_error_flags
                    .iter()
                    .any(|x| x.contains(&MotorErrorFlag::OverCurrent))
                {
                    Ok(BoardStatus::OverCurrentError)
                } else if flags.motor_error_flags.iter().any(|x| {
                    x.contains(&MotorErrorFlag::LowBusVoltage)
                        || x.contains(&MotorErrorFlag::DriverFault)
                }) {
                    Ok(BoardStatus::BusVoltageError)
                } else if flags
                    .homing_error_flags
                    .contains(&HomingErrorFlag::IndexSearchFail)
                {
                    Ok(BoardStatus::IndexError)
                } else if flags
                    .homing_error_flags
                    .contains(&HomingErrorFlag::ZeroingFail)
                {
                    Ok(BoardStatus::ZeroingError)
                } else if flags
                    .homing_error_flags
                    .contains(&HomingErrorFlag::AxisSensorReadFail)
                {
                    Ok(BoardStatus::SensorError)
                } else {
                    Ok(BoardStatus::InitError)
                }
            }
        }
    }
}

