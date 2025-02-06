use std::{
    error::Error,
    f32::consts::PI,
    thread,
    time::{Duration, SystemTime},
};

use poulpe_ethercat_grpc::PoulpeRemoteClient;

use poulpe_ethercat_controller::state_machine::{
    parse_homing_error_flags, parse_motor_error_flags, parse_state_from_status_word,
    parse_status_word, CiA402State,
};

// takes the salve id as argument
// and moves the motor in a sinusoidal motion
fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // args
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        log::error!("Usage:\n{}  <id> \nor \n{} slave_name", args[0], args[0]);
        return Err("Invalid number of arguments".into());
    }
    
    // first element is the nema or the id
    let client = if let Ok(id) = args[1].parse::<u16>() {
        log::info!("Connecting to the slave with id: {}", id);
        PoulpeRemoteClient::connect(
            "http://127.0.0.1:50098".parse()?,
            vec![id],
            Duration::from_secs_f32(0.001),
        )
    } else {
        let name = args[1].clone();
        log::info!("Connecting to the slave with name: {}", name);
        PoulpeRemoteClient::connect_with_name(
            "http://127.0.0.1:50098".parse()?,
            vec![name],
            Duration::from_secs_f32(0.001),
        )
    }.map_err(|e| {
        log::error!("Failed to connect to the server: {}", e);
        e
    })?;
    
    let id = id.unwrap();
    let name = name.unwrap();
    log::info!("Slave id: {}", id);
    log::info!("Slave name: {}", name);

    thread::sleep(Duration::from_secs(1));

    let state = client.get_cia402_state(id).unwrap();
    let cia_state: CiA402State = parse_state_from_status_word(state as u16);
    let status_bits = parse_status_word(state as u16);
    log::info!(
        "{}: Board State: {:?}\nStatus bits [{:?}]",
        name,
        cia_state,
        status_bits
    );
    let error_codes = client.get_error_codes(id).unwrap();
    let homing_error_flags = parse_homing_error_flags((error_codes[0] as u16).to_le_bytes());
    log::info!("{}: homing Error flags: {:?}", name, homing_error_flags);
    for (i, e) in error_codes.iter().enumerate().skip(1) {
        let motor_error = parse_motor_error_flags((*e as u16).to_le_bytes());
        log::info!("{}: motor {} | Error flags: {:?}", name, i, motor_error);
    }
    Ok(())
}
