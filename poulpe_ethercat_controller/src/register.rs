pub(crate) enum PdoRegister {
    State,
    OrbitaType,

    TroqueState,
    TargetPosition,
    VelocityLimit,
    TorqueLimit,

    TroqueOn,
    PositionActualValue,
    VelocityActualValue,
    TorqueActualValue,
    AxisSensorActualValue,
}

impl PdoRegister {
    pub(crate) fn name(&self) -> &'static str {
        match *self {
            PdoRegister::State => "state",
            PdoRegister::OrbitaType => "type",
            PdoRegister::TroqueState => "torque_state",
            PdoRegister::TroqueOn => "torque_enabled",
            PdoRegister::TargetPosition => "target",
            PdoRegister::PositionActualValue => "position",
            PdoRegister::VelocityActualValue => "velocity",
            PdoRegister::TorqueActualValue => "torque",
            PdoRegister::AxisSensorActualValue => "axis_sensor",
            PdoRegister::VelocityLimit => "velocity_limit",
            PdoRegister::TorqueLimit => "torque_limit",
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
    pub fn from_u8(value: u8) -> BoardStatus {
        match value {
            0 => BoardStatus::Ok,
            1 => BoardStatus::InitError,
            2 => BoardStatus::SensorError,
            3 => BoardStatus::IndexError,
            4 => BoardStatus::ZeroingError,
            5 => BoardStatus::OverTemperatureError,
            6 => BoardStatus::OverCurrentError,
            7 => BoardStatus::BusVoltageError,
            20 => BoardStatus::Init,
            100 => BoardStatus::HighTemperatureState,
            255 => BoardStatus::Unknown,
            _ => BoardStatus::Unknown,
        }
    }
}
