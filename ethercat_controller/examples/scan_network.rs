use ethercat_controller::EtherCatController;
use log;
use std::time::Duration;

fn main() {
    env_logger::init();

    let id: u16 = 0;

    log::info!("Creating the EtherCAT master");
    let ec = EtherCatController::open(0, Duration::from_millis(2), 1000, 500, 1000).unwrap();

    log::info!("Waiting for EtherCAT master to be ready");
    let ec = ec.wait_for_ready();
    log::info!("EtherCAT master is ready");


    log::info!("---------------------------");
    log::info!("Scanning network");
    ec.get_slave_ids().iter().for_each(|slave_id| {
        log::info!("Slave ID: {}, name: {}", slave_id, ec.get_slave_name(*slave_id).unwrap());
    });
}
