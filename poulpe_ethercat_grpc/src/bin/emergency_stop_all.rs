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

    let mut client = match PoulpeRemoteClient::connect(
        "http://127.0.0.1:50098".parse()?,
        vec![0],
        Duration::from_secs_f32(0.001),
    ) {
        Ok(client) => client,
        Err(e) => {
            log::error!("Failed to connect to the server: {}", e);
            return Err(e.into());
        }
    };

    let (all_ids, all_names) = client.get_poulpe_ids_sync()?;

    // show asscoiated names
    all_ids.iter().for_each(|id|{
        log::info!("id: {}, name: {}", id, all_names[*id as usize]);
    });

    all_ids.iter().for_each(|id|{
        client.emergency_stop(*id);
        log::info!("Emergency stop for id: {}", id);
        thread::sleep(Duration::from_millis(100));
    });
    Ok(())
}