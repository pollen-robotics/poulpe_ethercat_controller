use std::{
    error::Error,
    f32::consts::PI,
    thread,
    time::{Duration, SystemTime},
};

use poulpe_ethercat_controller::state_machine::CiA402ModeOfOperation;
use poulpe_ethercat_grpc::PoulpeRemoteClient;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let passiv_id = 1;
    let active_id = 0;

    let mut client = match PoulpeRemoteClient::connect(
        "http://127.0.0.1:50098".parse()?,
        vec![active_id, passiv_id],
        Duration::from_millis(1),
    ) {
        Ok(client) => client,
        Err(e) => {
            log::error!("Failed to connect to the server: {}", e);
            return Err(e.into());
        }
    };

    log::info!("Turn off slave {}", passiv_id);
    client.turn_off(passiv_id);

    client.set_velocity_limit(active_id, vec![1.0; 3]);
    client.set_torque_limit(active_id, vec![1.0; 3]);
    client.set_mode_of_operation(active_id, CiA402ModeOfOperation::ProfilePositionMode as u32);

    client.set_target_position(active_id, vec![0.0; 3]);
    log::info!("Turn on slave {}", active_id);
    client.turn_on(active_id);

    let t0 = SystemTime::now();

    let amp = 1.0;
    let freq = 0.2;

    thread::sleep(Duration::from_secs(1));

    loop {
        let actual_position = client.get_position_actual_value(passiv_id).unwrap();
        let actual_position_active = client.get_position_actual_value(active_id).unwrap();
        let target_position_active = client.get_target_position(active_id).unwrap();

        // let t = t0.elapsed().unwrap().as_secs_f32();
        // let target_position = amp * (2.0 * PI * freq * t).sin();
        // let target_position = target_position as u32;

        log::info!(
            "P0: {:?}\tP1: {:?},\tErr {}",
            actual_position[0] * -15.0,
            actual_position_active[0],
            actual_position[0] - actual_position_active[0] / -15.0
        );

        client.set_target_position(
            active_id,
            vec![(actual_position[0]) * -15.0; actual_position_active.len()],
        );
        thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}
