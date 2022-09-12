use std::{
    error::Error,
    f32::consts::PI,
    thread,
    time::{Duration, SystemTime},
};

use epos_ethercat_multiplexer::EposRemoteClient;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let passiv_id = 2;
    let active_id = 1;

    let mut client = EposRemoteClient::connect(
        "http://127.0.0.1:50098".parse()?,
        vec![active_id, passiv_id],
        Duration::from_millis(1),
    );

    log::info!("Turn off slave {}", passiv_id);
    client.turn_off(passiv_id);

    log::info!("Turn on slave {}", active_id);
    client.turn_on(active_id);

    // let t0 = SystemTime::now();
    // let amp = 45.0;
    // let freq = 0.5;

    thread::sleep(Duration::from_secs(1));

    loop {
        let actual_position = client.get_position_actual_value(passiv_id);
        log::info!("Pos: {} ", actual_position.to_degrees());

        // let t = t0.elapsed().unwrap().as_secs_f32();
        // let target_position = amp * (2.0 * PI * freq * t).sin();
        // log::info!("Pos: {} Target: {}", actual_position.to_degrees(), target_position);

        client.set_target_position(active_id, actual_position);
        thread::sleep(Duration::from_millis(1));
    }
}
