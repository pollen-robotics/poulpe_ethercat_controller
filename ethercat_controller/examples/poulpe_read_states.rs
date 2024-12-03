use ethercat_controller::EtherCatController;
use log;
use std::env;
use std::time::Duration;
fn main() {
    env_logger::init();

    // get the id from the argument
    let args: Vec<_> = env::args().collect();
    let id: u16 = match args.len() {
        2 => match &args[1].parse() {
            Ok(id) => *id,
            Err(_) => {
                log::error!("invalid slave id");
                println!("usage: {} slave_id", env!("CARGO_PKG_NAME"));
                return;
            }
        },
        _ => {
            log::error!("no slave id provided");
            println!("usage: {} slave_id", env!("CARGO_PKG_NAME"));
            return;
        }
    };

    log::info!("Loading the controller");
    let ec = EtherCatController::open(0, Duration::from_millis(2), 1000, 500, 1000).unwrap();

    log::info!("Waiting for controller to be ready");
    let ec = ec.wait_for_ready();
    log::info!("Controller is ready");

    std::thread::sleep(Duration::from_secs(1));
    // send switch on command
    ec.set_pdo_register(id, &"controlword".into(), 0, vec![0b0111, 0]);

    std::thread::sleep(Duration::from_secs(1));
    // ec.set_pdo_register(id, &"controlword".into(), 0, vec![0b1111, 0]);

    loop {
        let positions = ec
            .get_pdo_registers(id, &"actual_position".into())
            .unwrap()
            .iter()
            .map(|x| f32::from_le_bytes(x[0..4].try_into().unwrap()))
            .collect::<Vec<f32>>();
        let velocities = ec
            .get_pdo_registers(id, &"actual_velocity".into())
            .unwrap()
            .iter()
            .map(|x| f32::from_le_bytes(x[0..4].try_into().unwrap()))
            .collect::<Vec<f32>>();
        let axis = ec
            .get_pdo_registers(id, &"actual_axis_position".into())
            .unwrap()
            .iter()
            .map(|x| f32::from_le_bytes(x[0..4].try_into().unwrap()))
            .collect::<Vec<f32>>();
        let board_temperature = ec
            .get_pdo_registers(id, &"board_temperatures".into())
            .unwrap()
            .iter()
            .map(|x| f32::from_le_bytes(x[0..4].try_into().unwrap()))
            .collect::<Vec<f32>>();

        let motor_temperature = ec
            .get_pdo_registers(id, &"motor_temperatures".into())
            .unwrap()
            .iter()
            .map(|x| f32::from_le_bytes(x[0..4].try_into().unwrap()))
            .collect::<Vec<f32>>();
        log::info!(
            "Pos: {:?}, \t Vel: {:?},\t Axis: {:?}, \t Board Temp: {:?}, \t Motor Temp: {:?}",
            positions,
            velocities,
            axis,
            board_temperature,
            motor_temperature
        );
    }
}
