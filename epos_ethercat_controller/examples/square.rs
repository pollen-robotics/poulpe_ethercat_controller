use std::{env, error::Error, f32::consts::PI, time::{SystemTime, Duration}, thread::sleep};

use epos_ethercat_controller::EposController;

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
    let slave_id: u16 = args[2].parse()?;
    let amp: f32 = args[3].parse()?;
    let freq: f32 = args[4].parse()?;

    let epos_controller = EposController::connect(filename, 0_u32)?;

    log::info!("Setup slave {}", slave_id);
    epos_controller.setup(slave_id);

    log::info!("Turn on slave {}", slave_id);
    epos_controller.turn_on(slave_id, true);

    let t0 = SystemTime::now();

    let offset = amp;

    loop {
        epos_controller.set_target_position(slave_id, 0);
        sleep(Duration::from_secs(2));

        epos_controller.set_target_position(slave_id, 50);
        sleep(Duration::from_secs(2));
    }
}
