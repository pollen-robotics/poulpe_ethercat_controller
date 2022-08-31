use std::{env, io, thread, time::Duration};

use ethercat_controller::*;

fn main() -> Result<(), io::Error>{
    env_logger::init();

    let args: Vec<_> = env::args().collect();
    let filename = match args.len() {
        2 => &args[1],
        _ => {
            println!("usage: {} ESI-FILE", env!("CARGO_PKG_NAME"));
            return Ok(());
        }
    };

    let c = EtherCatController::open(filename, 0_u32)?;

    // loop {
    //     let pos = (
    //         c.get_pdo_position_actual_value(Slave::Id0),
    //         c.get_pdo_position_actual_value(Slave::Id1),
    //         c.get_pdo_position_actual_value(Slave::Id2),
    //     );
    //     println!("{:?}", pos);

    //     thread::sleep(Duration::from_millis(1));
    // }

    Ok(())
}