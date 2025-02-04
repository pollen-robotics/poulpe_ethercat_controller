use std::{
    error::Error,
    f32::consts::PI,
    thread,
    time::{Duration, SystemTime},
};

use poulpe_ethercat_grpc::client::PoulpeIdClient;
use poulpe_ethercat_grpc::PoulpeRemoteClient;

// takes the salve id as argument
// and moves the motor in a sinusoidal motion
fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let server_address = "http://127.0.0.1:50098";

    // read all slave ids and names in the network
    let id_client = PoulpeIdClient::new(server_address.parse().unwrap());
    let (all_ids, all_names) = id_client.get_slaves()?;
    // show asscoiated names
    all_ids.iter().for_each(|id| {
        log::info!("id: {}, name: {}", id, all_names[*id as usize]);
    });

    // creat ehe poulpe control client
    let mut client = match PoulpeRemoteClient::connect(
        server_address.parse()?,
        vec![],
        Duration::from_secs_f32(0.001),
    ) {
        Ok(client) => client,
        Err(e) => {
            log::error!("Failed to connect to the server: {}", e);
            return Err(e.into());
        }
    };


    all_ids.iter().for_each(|id| {
        client.emergency_stop(*id);
        log::info!("Emergency stop for id: {}", id);
        thread::sleep(Duration::from_millis(100));
    });
    Ok(())
}
