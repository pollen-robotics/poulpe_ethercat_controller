use std::{env, error::Error};

use epos_ethercat_controller::{Config, EposController};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args: Vec<_> = env::args().collect();
    if args.len() != 4 {
        println!(
            "usage: {} CONFIG PASSIV-ID ACTIVE-ID",
            env!("CARGO_PKG_NAME")
        );
        return Ok(());
    }

    let config_path = &args[1];
    let passive_id: u16 = args[2].parse()?;
    let active_id: u16 = args[2].parse()?;

    let config = Config::from_yaml(config_path)?;
    let epos_controller = EposController::connect(config)?;

    for slave_id in vec![passive_id, active_id] {
        log::info!("Setup slave {}", slave_id);
        epos_controller.setup(slave_id);
    }

    epos_controller.turn_off(passive_id);
    epos_controller.turn_on(active_id, true);

    loop {
        let pos = epos_controller.get_position_actual_value(passive_id);
        let vel = epos_controller.get_velocity_actual_value(passive_id);
        let torque = epos_controller.get_torque_actual_value(passive_id);

        log::info!("{} {} {}", pos, vel, torque);

        epos_controller.set_target_position(active_id, pos);

        epos_controller.wait_for_next_cycle();
    }
}
