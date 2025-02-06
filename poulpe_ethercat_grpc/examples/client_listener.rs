use std::{
    error::Error,
    f32::consts::PI,
    thread,
    time::{Duration, SystemTime},
};

use poulpe_ethercat_grpc::PoulpeRemoteClient;

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
    }
    .map_err(|e| {
        log::error!("Failed to connect to the server: {}", e);
        e
    })?;

    let id = client.ids[0];
    let name = client.names[0].clone();
    log::info!("Slave id: {}", id);
    log::info!("Slave name: {}", name);

    let t0 = SystemTime::now();

    let amp = 1.0;
    let freq = 0.2;

    thread::sleep(Duration::from_secs(2));

    let ids = client.get_poulpe_ids_sync()?;
    log::info!("Slave ids in network: {:?}", ids);

    loop {
        let complient = client.is_on(id).unwrap();
        let target_position = client.get_target_position(id).unwrap();
        let current_position = client.get_position_actual_value(id).unwrap();
        log::info!(
            "Compliant: {:?},\t Target position: {:?},\t Current position: {:?}",
            complient,
            target_position,
            current_position
        );
        thread::sleep(Duration::from_secs_f32(0.001));
    }
    Ok(())
}
