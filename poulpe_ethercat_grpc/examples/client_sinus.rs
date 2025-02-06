use std::{
    error::Error,
    f32::consts::PI,
    thread,
    time::{Duration, SystemTime},
};

use poulpe_ethercat_controller::state_machine::CiA402ModeOfOperation;
use poulpe_ethercat_grpc::PoulpeRemoteClient;

// takes the salve id or slave name as argument
// and moves the motor in a sinusoidal motion
fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // args
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        log::error!("Usage:\n{}  <id> \nor \n{} slave_name", args[0], args[0]);
        return Err("Invalid number of arguments".into());
    }

    // if the first argument is a number, it is the slave id
    // otherwise it is the slave name
    let mut client = if let Ok(id) = args[1].parse::<u16>() {
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

    thread::sleep(Duration::from_millis(100));
    match client.get_mode_of_operation(id) {
        Ok(mode) => {
            log::info!("Mode of operation: {:?}", mode);
            if mode != CiA402ModeOfOperation::ProfilePositionMode as u32 {
                log::info!("Setting mode of operation to ProfilePositionMode");
                client.turn_off(id);
                while client.is_on(id).unwrap() {
                    thread::sleep(Duration::from_millis(100));
                }
                client.set_mode_of_operation(id, CiA402ModeOfOperation::ProfilePositionMode as u32);
            }
        }
        Err(e) => log::error!("Failed to get mode of operation:"),
    }

    log::info!("Turn on slave {}", id);
    client.turn_on(id);

    let amp = 1.0;
    let freq = 0.2;

    thread::sleep(Duration::from_secs(2));

    let ids = client.get_poulpe_ids_sync()?;
    log::info!("Slave ids in network: {:?}", ids);

    client.set_velocity_limit(id, vec![1.0; 3]);
    client.set_torque_limit(id, vec![1.0; 3]);

    let mut t1 = SystemTime::now();
    let mut max_t1 = 0.0;
    client.set_target_position(id, vec![0.0; 3]);

    let t0 = SystemTime::now();
    loop {
        let actual_position = client.get_position_actual_value(id).unwrap();
        let actual_velocity = client.get_velocity_actual_value(id).unwrap();
        let actual_torque = client.get_torque_actual_value(id).unwrap();
        let state = client.get_state(id);

        log::info!(
            "{:?}/{:?} state {:?}, pos: {:?}\tvel: {:?}\ttorque: {:?}",
            t1.elapsed().unwrap(),
            max_t1 / 1000.0,
            state,
            actual_position,
            actual_velocity,
            actual_torque
        );
        if t1.elapsed().unwrap().as_micros() as f32 > max_t1 {
            max_t1 = t1.elapsed().unwrap().as_micros() as f32;
        }
        t1 = SystemTime::now();

        let t = t0.elapsed().unwrap().as_secs_f32();
        let target_position = amp * (2.0 * PI * freq * t).sin();

        client.set_target_position(id, vec![target_position; 3]);
        thread::sleep(Duration::from_secs_f32(0.001));
        client.turn_on(id);
    }
    Ok(())
}
