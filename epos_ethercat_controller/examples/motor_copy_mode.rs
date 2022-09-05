use std::{env, io};

use epos_ethercat_controller::EposController;

fn main() -> Result<(), io::Error> {
    env_logger::init();

    let args: Vec<_> = env::args().collect();
    let filename = match args.len() {
        2 => &args[1],
        _ => {
            println!("usage: {} ESI-FILE", env!("CARGO_PKG_NAME"));
            return Ok(());
        }
    };

    let epos_controller = EposController::connect(filename, 0_u32)?;

    for slave_id in vec![0, 1, 2] {
        log::info!("Setup slave {}", slave_id);
        epos_controller.setup(slave_id);
    }

    epos_controller.turn_off(0);
    epos_controller.turn_on(1, true);

    loop {
        let pos = epos_controller.get_position_actual_value(0);
        let vel = epos_controller.get_velocity_actual_value(0);
        let torque = epos_controller.get_torque_actual_value(0);

        log::info!("{} {} {}", pos, vel, torque);

        epos_controller.set_target_position(1, pos);

        epos_controller.wait_for_next_cycle();
    }
}
