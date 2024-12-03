#[derive(FromPrimitive, Debug, PartialEq)]
pub enum StatusBit {
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

#[derive(FromPrimitive, Debug, PartialEq)]
pub enum ControlWord {
    Shutdown,
    SwitchOn,       // the same as DisableOperation
    DisableVoltage, // NOT USED
    EnableOperation,
    DisableOperation,
    QuickStop,
    FaultReset,
    Unknown,
}

impl ControlWord {
    pub fn to_u16(&self) -> u16 {
        match self {
            ControlWord::Shutdown => 0b0110,
            ControlWord::SwitchOn => 0b0111,
            ControlWord::DisableVoltage => 0b0000, // NOT USED
            ControlWord::EnableOperation => 0b1111,
            ControlWord::DisableOperation => 0b0111,
            ControlWord::QuickStop => 0b0010,
            ControlWord::FaultReset => 0b10000000,
            ControlWord::Unknown => 0b0,
        }
    }
}

// Error codes for the motors, we will have one error code per motor
// - None - no error
// - ConfigFail - error during the configuration of the motor
// - MotorAlignFail - error during the motor alignment
// - HighTemperatureWarning - warning for high temperature
// - OverTemperature - error due to the temperature being too high
// - OverCurrent - error due to the current being too high
// - LowBusVoltage - error due to the bus voltage being too low
// - CommunicationFail - error due to communication failure with the motor driver
#[derive(FromPrimitive, PartialEq, Clone, Copy, Debug)]
pub enum MotorErrorFlag {
    // None = 0,
    ConfigFail = 0,
    MotorAlignFail = 1,
    HighTemperatureWarning = 2,
    OverTemperatureMotor = 3,
    OverTemperatureBoard = 4,
    OverCurrent = 5,
    LowBusVoltage = 6,
    DriverFault = 7,
}

// Error codes for the homing procedure
// - None - no error
// - AxisSensorReadFail - error during the reading of the axis sensor
// - ZeroingFail - error during the zeroing of the axis positions
// - IndexSearchFail - error during the search of the index (only orbita3d)
#[derive(FromPrimitive, PartialEq, Clone, Copy, Debug)]
pub enum HomingErrorFlag {
    // None = 0,
    AxisSensorReadFail = 0,
    MotorMovementCheckFail = 1,
    AxisSensorAlignFail = 2,
    ZeroingFail = 3,
    IndexSearchFail = 4,
    CommunicationFail = 5,
}

#[derive(FromPrimitive, PartialEq, Clone, Copy, Debug)]
#[repr(u16)]
pub enum CiA402State {
    NotReadyToSwitchOn = 0b00000000, // initialisation and test of the drive is not yet completed
    SwitchOnDisabled = 0b01000000,   // init passed successfully
    ReadyToSwitchOn = 0b00100001, // init sucess + switch off received - (more or less saying that the EtherCAT is connected)
    SwitchedOn = 0b00100011,      // init sucess + switch on received
    //  - in our case we send operation enabled and switch on at the same time, so we dont really use this state
    OperationEnabled = 0b00110111, // switched on + enable operation received
    QuickStopActive = 0b00000111, // quick stop procedure going to Switch_on_disabled state ( we don't use quick stop )
    FaultReactionActive = 0b00011111, // fault reaction going to Fault state
    Fault = 0b00001000,           // fault state
}

#[derive(Debug)]
pub struct ErrorFlags {
    pub motor_error_flags: Vec<Vec<MotorErrorFlag>>,
    pub homing_error_flags: Vec<HomingErrorFlag>,
}

pub fn parse_status_word(status: u16) -> Vec<StatusBit> {
    let mut status_bits = Vec::new();
    for i in 0..16 {
        if status & (1 << i) != 0 {
            status_bits.push(num::FromPrimitive::from_u8(i as u8).unwrap());
        }
    }
    status_bits
}

pub fn parse_motor_error_flags(error: [u8; 2]) -> Vec<MotorErrorFlag> {
    let motor_error = u16::from_le_bytes(error);
    let mut error_flags = Vec::new();
    for i in 0..16 {
        if motor_error & (1 << i) != 0 {
            error_flags.push(num::FromPrimitive::from_u16(i as u16).unwrap());
        }
    }
    error_flags
}

pub fn parse_homing_error_flags(error: [u8; 2]) -> Vec<HomingErrorFlag> {
    let homming_error = u16::from_le_bytes(error);
    let mut error_flags = Vec::new();
    for i in 0..16 {
        if homming_error & (1 << i) != 0 {
            error_flags.push(num::FromPrimitive::from_u16(i as u16).unwrap());
        }
    }
    error_flags
}

pub fn parse_state_from_status_word(status: u16) -> CiA402State {
    num::FromPrimitive::from_u16(status).unwrap()
}

pub fn parse_state_from_status_bits(
    status_bits: Vec<StatusBit>,
) -> Result<CiA402State, Box<dyn std::error::Error>> {
    let mut state = 0;
    for bit in status_bits {
        state |= 1 << bit as u16;
    }

    // remove the manufacturer specific bits
    // bits 8, 14 and 15
    {
        state = state & 0b0011111011111111;
    }

    // remove the warning bit
    state = state & 0b1111111101111111;

    match num::FromPrimitive::from_u16(state) {
        Some(s) => Ok(s),
        None => Err("Invalid state".into()),
    }
}
