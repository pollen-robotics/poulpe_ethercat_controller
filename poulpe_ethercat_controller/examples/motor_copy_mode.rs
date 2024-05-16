use std::{env, io};

use ethercat_controller::Config;
use poulpe_ethercat_controller::PoulpeController;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<_> = env::args().collect();
    let filename = match args.len() {
        2 => &args[1],
        _ => {
            println!("usage: {} ESI-FILE", env!("CARGO_PKG_NAME"));
            return Ok(());
        }
    };

    let filename = &args[1];

    let pouple_controller = PoulpeController::connect(filename)?;

    for slave_id in vec![0, 1] {
        log::info!("Setup slave {}", slave_id);
        pouple_controller.setup(slave_id);
    }
    let n_axis = pouple_controller.get_type(0) as usize;

    pouple_controller.set_torque(0u32, true)?;
    pouple_controller.set_torque(1u32, false)?;

    pouple_controller.set_torque_limit(0, vec![1.0; n_axis])?; // torque limit at 40%
    pouple_controller.set_velocity_limit(0, vec![1.0; n_axis])?; // velocity limit at 10%

    loop {
        let pos0 = match pouple_controller.get_current_position(0) {
            Ok(Some(pos)) => pos,
            _ => {
                log::error!("Error getting position!");
                vec![0.0; 2]
            }
        };
        let pos1 = match pouple_controller.get_current_position(1) {
            Ok(Some(pos)) => pos,
            _ => {
                log::error!("Error getting position!");
                vec![0.0; 2]
            }
        };

        log::info!(
            "P0 ({}d): {:?},\t P1 ({}d): {:?}",
            pos0.len(),
            pos0[0],
            pos1.len(),
            pos1[0]
        );

        pouple_controller.set_target_position(0, vec![pos1[0]; n_axis])?;

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
