use ethercat_controller::EtherCatController;
use log;
use std::time::Duration;

fn main() {
    env_logger::init();

    let id: u16 = 0;

    log::info!("Loading the controller");
    let ec = EtherCatController::open(0, Duration::from_millis(2)).unwrap();

    log::info!("Waiting for controller to be ready");
    let ec = ec.wait_for_ready();
    log::info!("Controller is ready");

    let t0 = std::time::Instant::now();
    loop {
        let status = ec.get_pdo_register(id, &"state".into(), 0);
        let orbita_type = ec.get_pdo_register(id, &"type".into(), 0);
        log::info!("Status: {:?}, Type: {:?}", status, orbita_type);

        let positions = ec
            .get_pdo_registers(id, &"position".into())
            .unwrap()
            .iter()
            .map(|x| f32::from_le_bytes(x[0..4].try_into().unwrap()))
            .collect::<Vec<f32>>();
        let velocities = ec
            .get_pdo_registers(id, &"velocity".into())
            .unwrap()
            .iter()
            .map(|x| f32::from_le_bytes(x[0..4].try_into().unwrap()))
            .collect::<Vec<f32>>();
        let torques = ec
            .get_pdo_registers(id, &"torque".into())
            .unwrap()
            .iter()
            .map(|x| f32::from_le_bytes(x[0..4].try_into().unwrap()))
            .collect::<Vec<f32>>();
        let axis = ec
            .get_pdo_registers(id, &"axis_sensor".into())
            .unwrap()
            .iter()
            .map(|x| f32::from_le_bytes(x[0..4].try_into().unwrap()))
            .collect::<Vec<f32>>();
        let target_position = ec
            .get_pdo_registers(id, &"target".into())
            .unwrap()
            .iter()
            .map(|x| f32::from_le_bytes(x[0..4].try_into().unwrap()))
            .collect::<Vec<f32>>();
        log::info!(
            "{:?}\t Tar: {:?},\t Pos: {:?}, \t Vel: {:?},\t  Tor: {:?},\t Axis: {:?}",
            t0.elapsed().as_millis(),
            target_position,
            positions,
            velocities,
            torques,
            axis
        );

        let time = t0.elapsed().as_millis() as f32;
        let sin_target = 0.5 * f32::sin(0.001 * time);
        log::debug!("{}, {}", time, sin_target);
        // enable the first motor (by setring 1 to the 0th bit of the torque_state register)
        ec.set_pdo_register(id, &"torque_state".into(), 0, vec![0b11]);
        // set the target position to the first motor (index 0)
        ec.set_pdo_register(id, &"target".into(), 0, sin_target.to_le_bytes().to_vec());
        ec.set_pdo_register(id, &"target".into(), 1, sin_target.to_le_bytes().to_vec());
        // set the torque and velocity limit
        ec.set_pdo_register(
            id,
            &"velocity_limit".into(),
            0,
            1.0f32.to_le_bytes().to_vec(),
        );
        // set the torque and velocity limit
        ec.set_pdo_register(
            id,
            &"velocity_limit".into(),
            1,
            1.0f32.to_le_bytes().to_vec(),
        );
        ec.set_pdo_register(id, &"torque_limit".into(), 0, 1.0f32.to_le_bytes().to_vec());
        ec.set_pdo_register(id, &"torque_limit".into(), 1, 1.0f32.to_le_bytes().to_vec());

        // std::thread::sleep(Duration::from_millis(1));
    }
}
