use core::net;
use ethercat_controller::EtherCatController;
use log;
use std::env;
use std::time::Duration;

// control words na mode of operation
// see more info in poulpe_ethercat_controller::state_machine
const SWITCH_ON: u16 = 0b0111;
const SHUT_DOWN: u16 = 0b0110;
const ENABLE_OPERATION: u16 = 0b1111;
const POSITION_MODE: u8 = 1;

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
    ec.set_pdo_register(
        id,
        &"controlword".into(),
        0,
        SHUT_DOWN.to_le_bytes().to_vec(),
    );

    std::thread::sleep(Duration::from_secs(1));
    // set the mode of operation to profile position mode
    ec.set_pdo_register(id, &"mode_of_operation".into(), 0, vec![POSITION_MODE]);

    let t0 = std::time::Instant::now();
    std::thread::sleep(Duration::from_secs(1));

    let p = ec.get_pdo_registers(id, &"actual_position".into()).unwrap();
    let n_axis = p.len();

    // send switch on command
    ec.set_pdo_register(
        id,
        &"controlword".into(),
        0,
        SWITCH_ON.to_le_bytes().to_vec(),
    );
    loop {
        let status = ec.get_pdo_register(id, &"statusword".into(), 0);
        let orbita_type = ec.get_pdo_register(id, &"actuator_type".into(), 0);
        log::info!("Status: {:?}, Type: {:?}", status, orbita_type);

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
        let torques = ec
            .get_pdo_registers(id, &"actual_torque".into())
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
        let target_position = ec
            .get_pdo_registers(id, &"target_position".into())
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
            "{:?}\t Tar: {:?},\t Pos: {:?}, \t Vel: {:?},\t  Tor: {:?},\t Axis: {:?}, \t Board Temp: {:?}, \t Motor Temp: {:?}",
            t0.elapsed().as_millis(),
            target_position,
            positions,
            velocities,
            torques,
            axis,
            board_temperature,
            motor_temperature
        );

        let time = t0.elapsed().as_millis() as f32;
        let sin_target = 0.5 * f32::sin(0.001 * time);
        log::debug!("{}, {}", time, sin_target);
        // enable the first motor (by setring 1 to the 0th bit of the torque_state register)
        ec.set_pdo_register(
            id,
            &"controlword".into(),
            0,
            ENABLE_OPERATION.to_le_bytes().to_vec(),
        );
        // set the target position to the first motor (index 0)
        ec.set_pdo_registers(
            id,
            &"target_position".into(),
            vec![sin_target.to_le_bytes().to_vec(); n_axis],
        );
        // set the torque and velocity limit
        ec.set_pdo_registers(
            id,
            &"velocity_limit".into(),
            vec![1.0f32.to_le_bytes().to_vec(); n_axis],
        );
        ec.set_pdo_registers(
            id,
            &"torque_limit".into(),
            vec![1.0f32.to_le_bytes().to_vec(); n_axis],
        );
    }
}
