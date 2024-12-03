use std::{env, error::Error, f32::consts::PI, time::SystemTime};

use ethercat_controller::Config;
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

    log::info!("Turn off slave {}", slave_id);
    pouple_controller.set_torque(slave_id, false)?;

    // sleep 2s
    std::thread::sleep(std::time::Duration::from_secs(2));

    log::info!("Turn on slave {}", slave_id);
    pouple_controller.set_torque(slave_id, true)?;

    let no_axis = pouple_controller.get_type(slave_id) as usize;

    log::info!("setup the torque and velocity limits");
    pouple_controller.set_torque_limit(slave_id, vec![1.0; no_axis])?;
    pouple_controller.set_velocity_limit(slave_id, vec![1.0; no_axis])?;

    let mut t0 = SystemTime::now();
    let mut t1 = SystemTime::now();

    let mut max_t1 = 0.0;
    loop {
        let t = t0.elapsed().unwrap().as_secs_f32();

        let pos = match pouple_controller.get_current_position(slave_id) {
            Ok(Some(pos)) => pos,
            _ => {
                log::error!("Error getting position!");
                vec![0.0; 2]
            }
        };
        let vel = match pouple_controller.get_current_velocity(slave_id) {
            Ok(Some(vel)) => vel,
            _ => {
                log::error!("Error getting velocity!");
                vec![0.0; 2]
            }
        };
        let torque = match pouple_controller.get_current_torque(slave_id) {
            Ok(Some(torque)) => torque,
            _ => {
                log::error!("Error getting torque!");
                vec![0.0; 2]
            }
        };

        let target = amp * (2.0 * PI * freq * t).sin();
        pouple_controller.set_target_position(slave_id, vec![target; no_axis])?;

        let temps = match pouple_controller.get_temperatures(slave_id) {
            Ok(Some(temps)) => temps,
            _ => {
                log::error!("Error getting temperatures!");
                (vec![0.0; 2], vec![0.0; 2])
            }
        };

        let state = pouple_controller.get_status(slave_id)?;

        let error = [target - pos[0], target - pos[1]];

        log::info!(
            "{:?}/{:?}\t State: {:?}\t Pos: {:?}\t Vel: {:?}\t Torque: {:?}\t Error: {:?},\t Temperatures: {:?}",
            t1.elapsed().unwrap(),
            max_t1/1000.0,
            state,
            pos,
            vel,
            torque,
            error,
            temps
        );
        if t1.elapsed().unwrap().as_micros() as f32 > max_t1 {
            max_t1 = t1.elapsed().unwrap().as_micros() as f32;
        }
        t1 = SystemTime::now();

        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}
