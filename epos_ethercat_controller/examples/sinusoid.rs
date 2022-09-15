use std::{env, error::Error, f32::consts::PI, time::SystemTime};

use epos_ethercat_controller::{Config, EposController};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args: Vec<_> = env::args().collect();

    if args.len() != 5 {
        println!("usage: {} CONFIG SLAVE_ID AMP(DEG) FREQ", env!("CARGO_PKG_NAME"));
        return Ok(());
    }

    let config_path = &args[1];
    let slave_id: u16 = args[2].parse()?;
    let amp: f32 = args[3].parse()?;
    let amp = amp.to_radians();

    let freq: f32 = args[4].parse()?;

    let config = Config::from_yaml(config_path)?;
    let mut epos_controller = EposController::connect(config)?;

    log::info!("Setup slave {}", slave_id);
    epos_controller.setup(slave_id, true);

    log::info!("Turn on slave {}", slave_id);
    epos_controller.turn_on(slave_id, true);

    let t0 = SystemTime::now();

    let pos0 = epos_controller.get_position_actual_value(slave_id);

    loop {
        let t = t0.elapsed().unwrap().as_secs_f32();

        let pos = epos_controller.get_position_actual_value(slave_id);
        let vel = epos_controller.get_velocity_actual_value(slave_id);
        let torque = epos_controller.get_torque_actual_value(slave_id);

        let target = pos0 + amp * (2.0 * PI * freq * t).sin();
        epos_controller.set_target_position(slave_id, target);

        // epos_controller.set_target_position(slave_id, 0.0);

        let error = target as i32 - pos as i32;

        log::info!(
            "Pos: {} Vel: {} Torque: {} Error: {}",
            pos,
            vel,
            torque,
            error
        );

        epos_controller.wait_for_next_cycle();
    }
}
