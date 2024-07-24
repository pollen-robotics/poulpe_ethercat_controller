use std::{
    error::Error,
    f32::consts::PI,
    thread,
    time::{Duration, SystemTime},
};

use poulpe_ethercat_grpc::PoulpeRemoteClient;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let id: u16 = 0;

    let mut client = match PoulpeRemoteClient::connect(
        "http://127.0.0.1:50098".parse()?,
        vec![id],
        Duration::from_secs_f32(0.001),
    ){
        Ok(client) => client,
        Err(e) => {
            log::error!("Failed to connect to the server: {}", e);
            return Err(e.into());
        }
    };

    log::info!("Turn on slave {}", id);
    client.turn_on(id);

    let t0 = SystemTime::now();

    let amp = 1.0;
    let freq = 0.2;

    thread::sleep(Duration::from_secs(2));

    let ids = client.get_poulpe_ids_sync()?;
    log::info!("ids: {:?}", ids);

    client.set_velocity_limit(id, vec![1.0; 3]);
    client.set_torque_limit(id, vec![1.0; 3]);

    let mut t1 = SystemTime::now();
    let mut max_t1 = 0.0;
    client.set_target_position(id, vec![0.0; 3]);

    loop {
        let actual_position = client.get_position_actual_value(id).unwrap();
        let actual_velocity = client.get_velocity_actual_value(id).unwrap();
        let actual_torque = client.get_torque_actual_value(id).unwrap();
        let state = client.get_state(id);

        // log::info!(
        //     "{:?}/{:?} state {:?}, pos: {:?}\tvel: {:?}\ttorque: {:?}",
        //     t1.elapsed().unwrap(),
        //     max_t1/1000.0,
        //     state,
        //     actual_position,
        //     actual_velocity,
        //     actual_torque
        // );
        if t1.elapsed().unwrap().as_micros() as f32 > max_t1 {
            max_t1 = t1.elapsed().unwrap().as_micros() as f32;
        }
        t1 = SystemTime::now();

        let t = t0.elapsed().unwrap().as_secs_f32();
        let target_position = amp * (2.0 * PI * freq * t).sin();

        client.set_target_position(id, vec![target_position; 3]);
        thread::sleep(Duration::from_secs_f32(0.001));
    }
    Ok(())
}
