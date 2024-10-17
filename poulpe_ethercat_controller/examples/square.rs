use std::{
    env,
    error::Error,
    thread::sleep,
    time::{Duration, SystemTime},
};

use poulpe_ethercat_controller::PoulpeController;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args: Vec<_> = env::args().collect();

    if args.len() != 5 {
        println!(
            "usage: {} ESI-FILE SLAVE_ID AMP FREQ",
            env!("CARGO_PKG_NAME")
        );
        return Ok(());
    }

    let filename = &args[1];
    let slave_id: u32 = args[2].parse()?;
    let amp: f32 = args[3].parse()?;
    let freq: f32 = args[4].parse()?;

    let pouple_controller = PoulpeController::connect(filename)?;

    log::info!("Setup slave {}", slave_id);
    pouple_controller.setup(slave_id)?;

    log::info!("Turn on slave {}", slave_id);
    pouple_controller.set_torque(slave_id, true)?;

    let t0 = SystemTime::now();

    let no_axis = pouple_controller.get_type(slave_id) as usize;

    log::info!("setup the torque and velocity limits");
    pouple_controller.set_torque_limit(slave_id, vec![0.4; no_axis])?; // torque limit at 40%
    pouple_controller.set_velocity_limit(slave_id, vec![0.1; no_axis])?; // velocity limit at 10%

    loop {
        pouple_controller.set_target_position(slave_id, vec![0.0; no_axis])?;
        sleep(Duration::from_secs(2));

        log::info!(
            "Current position: {:?}",
            pouple_controller.get_current_position(slave_id)?
        );

        pouple_controller.set_target_position(slave_id, vec![3.14; no_axis])?;
        sleep(Duration::from_secs(2));
        log::info!(
            "Current position: {:?}",
            pouple_controller.get_current_position(slave_id)?
        );
    }
}
