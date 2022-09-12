use std::{env, error::Error, f32::consts::PI, time::SystemTime};

use epos_ethercat_controller::EposController;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args: Vec<_> = env::args().collect();

    if args.len() != 3 {
        println!("usage: {} ESI-FILE SLAVE_ID", env!("CARGO_PKG_NAME"));
        return Ok(());
    }

    let filename = &args[1];
    let slave_id: u16 = args[2].parse()?;

    let epos_controller = EposController::connect(filename, 0_u32)?;

    log::info!("Setup slave {}", slave_id);
    epos_controller.setup(slave_id);

    epos_controller.turn_on(slave_id, true);

    let t0 = SystemTime::now();
    let freq = 0.5;
    let amp = 45.0;

    loop {
        let t = t0.elapsed().unwrap().as_secs_f32();
        let target = amp * (2.0 * PI * freq * t).sin();

        epos_controller.set_target_position(slave_id, target.to_radians());

        let pos = epos_controller.get_position_actual_value(slave_id);
        // let absolute_encoder = epos_controller.get_absolute_encoder_value(slave_id);
        // let hall_sensor = epos_controller.get_hall_sensor_value(slave_id);
        // log::info!("Pos: {} Abs: {} Hall: {}", pos, absolute_encoder, hall_sensor);

        log::info!("{} {}", target, pos.to_degrees());

        epos_controller.wait_for_next_cycle();
    }
}
