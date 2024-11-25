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
