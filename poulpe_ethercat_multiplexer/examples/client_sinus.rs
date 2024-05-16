use std::{
    error::Error,
    f32::consts::PI,
    thread,
    time::{Duration, SystemTime},
};

use poulpe_ethercat_multiplexer::PoulpeRemoteClient;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let id: u16 = 0;

    let mut client = PoulpeRemoteClient::connect(
        "http://127.0.0.1:50098".parse()?,
        vec![id],
        Duration::from_millis(5),
    );

    log::info!("Turn off slave {}", id);
    client.turn_on(id);

    let t0 = SystemTime::now();

    let amp = 1.0;
    let freq = 0.2;

    thread::sleep(Duration::from_secs(1));

    client.set_velocity_limit(id, vec![1.0; 3]);
    client.set_torque_limit(id, vec![1.0; 3]);

    loop {
        let actual_position = client.get_position_actual_value(id).unwrap();
        let actual_velocity = client.get_velocity_actual_value(id).unwrap();
        let actual_torque = client.get_torque_actual_value(id).unwrap();
        let state = client.get_state(id);

        log::info!(
            "state {:?}, pos: {:?}\tvel: {:?}\ttorque: {:?}",
            state,
            actual_position,
            actual_velocity,
            actual_torque
        );

        let t = t0.elapsed().unwrap().as_secs_f32();
        let target_position = amp * (2.0 * PI * freq * t).sin();

        client.set_target_position(id, vec![target_position; actual_position.len()]);
        thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}
