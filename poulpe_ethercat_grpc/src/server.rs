use std::{
    env,
    f32::consts::E,
    mem::take,
    sync::Arc,
    time::{Duration, SystemTime},
};

use poulpe_ethercat_controller::PoulpeController;
use tokio::{sync::mpsc, time::{error::Elapsed, sleep}};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{transport::Server, Request, Response, Status, Streaming};

use poulpe_ethercat_grpc::pb::{
    poulpe_multiplexer_server::{PoulpeMultiplexer, PoulpeMultiplexerServer},
    PoulpeCommands, PoulpeIds, PoulpeState, PoulpeStates, StateStreamRequest
};

use prost_types::Timestamp;
#[derive(Debug)]
struct PoulpeMultiplexerService {
    controller: Arc<PoulpeController>,
}

fn get_state_for_id(controller: &PoulpeController, id: i32) -> Result<PoulpeState, Box<dyn std::error::Error>> {
    let slave_id = id as u32;

    // check if slave ready, if not dont read its state
    if controller.is_slave_ready(slave_id as u16) == false{
        log::error!("Slave (id: {}) not ready!", slave_id);
        return Err(("Slave not ready!").into());
    }

    Ok(PoulpeState {
        id,
        mode_of_operation : match controller.get_mode_of_operation_display(slave_id as u16) {
            Ok(mode) => mode as i32,
            _ => {
                log::error!("Failed to get mode of operation for slave {}", slave_id);
                return Err("Failed to get mode of operation".into());
            }
        },
        actual_position: match controller.get_current_position(slave_id) {
            Ok(Some(pos)) => pos,
            _ => {
                log::error!("Failed to get actual position for slave {}", slave_id);
                return Err("Failed to get actual position".into());
            }
        },
        actual_velocity: match controller.get_current_velocity(slave_id) {
            Ok(Some(vel)) => vel,
            _ => {
                log::error!("Failed to get actual velocity for slave {}", slave_id);
                return Err("Failed to get actual velocity".into());
            }
        },
        actual_torque: match controller.get_current_torque(slave_id) {
            Ok(Some(torque)) => torque,
            _ => {
                log::error!("Failed to get actual torque for slave {}", slave_id);
                return Err("Failed to get actual torque".into());
            }
        },
        axis_sensors: match controller.get_current_axis_sensors(slave_id) {
            Ok(Some(sensor)) => sensor,
            _ => {
                log::error!("Failed to get axis sensor for slave {}", slave_id);
                return Err("Failed to get axis sensor".into());
            }
        },
        axis_sensor_zeros: match controller.get_axis_sensor_zeros(slave_id) {
            Ok(Some(sensor)) => sensor,
            _ => {
                log::error!("Failed to get axis sensor zeros for slave {}", slave_id);
                return Err("Failed to get axis sensor zeros".into());
            }
        },
        board_temperatures: match controller.get_board_temperatures(slave_id) {
            Ok(Some(temps)) => temps,
            _ => {
                log::error!("Failed to get board temperatures for slave {}", slave_id);
                return Err("Failed to get board temperatures".into());
            }
        },
        motor_temperatures: match controller.get_motor_temperatures(slave_id) {
            Ok(Some(temps)) => temps,
            _ => {
                log::error!("Failed to get motor temperatures for slave {}", slave_id);
                return Err("Failed to get motor temperatures".into());
            }
        },
        requested_target_position: match controller.get_current_target_position(slave_id) {
            Ok(Some(pos)) => pos,
            _ => {
                log::error!(
                    "Failed to get requested target position for slave {}",
                    slave_id
                );
                return Err("Failed to get requested target position".into());
            }
        },
        requested_velocity_limit: match controller.get_current_velocity_limit(slave_id) {
            Ok(Some(velocity_limit)) => velocity_limit,
            _ => {
                log::error!(
                    "Failed to get requested velcity limit for slave {}",
                    slave_id
                );
                return Err("Failed to get requested  velcity limit".into());
            }
        },
        requested_torque_limit: match controller.get_current_torque_limit(slave_id) {
            Ok(Some(troque_limit)) => troque_limit,
            _ => {
                log::error!(
                    "Failed to get requested torque limit for slave {}",
                    slave_id
                );
                return Err("Failed to get requested torque limit".into());
            }
        },
        state: match controller.get_status(slave_id) {
            Ok(state) => state as u32,
            _ => {
                log::error!("Failed to get state for slave {}", slave_id);
                return Err("Failed to get state".into());
            }
        },
        error_codes: match controller.get_error_codes(slave_id) {
            Ok(error_codes) => error_codes.iter().map(|x| *x as i32).collect(),
            _ => {
                log::error!("Failed to get error codes for slave {}", slave_id);
                return Err("Failed to get error codes".into());
            }
        },
        compliant: match controller.is_torque_on(slave_id) {
            Ok(Some(state)) => state,
            _ => {
                log::error!("Failed to get torque state for slave {}", slave_id);
                return Err("Failed to get torque state".into());
            }
        },
        published_timestamp: Some(Timestamp::from(std::time::SystemTime::now())), 
    })
}

#[tonic::async_trait]
impl PoulpeMultiplexer for PoulpeMultiplexerService {
    async fn get_poulpe_ids(&self, _request: Request<()>) -> Result<Response<PoulpeIds>, Status> {
        let reply = PoulpeIds {
            ids: self
                .controller
                .get_slave_ids()
                .iter()
                .map(|&id| id as i32)
                .collect(),
            names: self
                .controller
                .get_slave_names()
        };

        Ok(Response::new(reply))
    }

    type GetStatesStream = ReceiverStream<Result<PoulpeStates, Status>>;

    async fn get_states(
        &self,
        request: Request<StateStreamRequest>,
    ) -> Result<Response<Self::GetStatesStream>, Status> {
        let (tx, rx) = mpsc::channel(2);

        let controller = self.controller.clone();

        log::info!("New client - update period of {}s", request.get_ref().update_period);

        tokio::spawn(async move {
            let request = request.get_ref();
            // fixed frequency
            let mut interval = tokio::time::interval(Duration::from_secs_f32(request.update_period));
    
            // state to be sent if no state is available
            // this is to avoid sending empty states
            let mut last_state = poule_empty_state();

            // DEBUGGING
            let mut nb_states = 0;
            let mut t_debug = tokio::time::Instant::now();
            // debug report sent time
            // first report will be displayed after this many seconds
            // and then every other will be after 30s
            let mut report_display_time = 5.0; // seconds
            loop {
                // Wait until the next tick
                interval.tick().await;

                let states = PoulpeStates {
                    states: request
                        .ids
                        .iter()
                        .map(|&id| 
                            match get_state_for_id(&controller, id){
                                Ok(state) => {
                                    last_state = state.clone();
                                    state
                                },
                                Err(e) => {
                                    log::error!("Failed to get state for slave {}: {}", id, e);
                                    last_state.clone()
                                }
                            }

                        )
                        .collect(),
                };
                if tx.send(Ok(states)).await.is_err() {
                    break;
                }

                // DEBUGGING
                // send the report to the user about the states 
                nb_states += 1;
                if t_debug.elapsed().as_secs_f32() > report_display_time {
                    log::info!("GRPC EtherCAT Slave {} | states sent {} req/s", request.ids[0], nb_states as f32/t_debug.elapsed().as_secs_f32());
                    nb_states = 0;
                    t_debug = tokio::time::Instant::now();
                    report_display_time = 30.0; // [secs] next report display 
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_commands(
        &self,
        request: Request<Streaming<PoulpeCommands>>,
    ) -> Result<Response<()>, Status> {
        let mut stream = request.into_inner();

        // DEBUGGING
        // measure the time for debugging
        let mut t_debug = tokio::time::Instant::now();
        // measure the number of commands sent and dropped
        let mut nb_commands = 0;
        let mut nb_dropped = 0;
        // measure the time between the commands
        let mut t_command = tokio::time::Instant::now();
        // the longest time between two commands
        let mut dt_command_max :f32 = 0.0;
        // debug report sent time
        // first report will be displayed after this many seconds
        // and then every other will be after 30s
        let mut report_display_time = 5.0; // seconds

        while let Some(Ok(req)) = stream.next().await {

            let mut slave_id:u32 = req.commands[0].id as u32;
            // check if the slave is ready and drop the command if not
            if self.controller.is_slave_ready(slave_id as u16) == false{
                log::error!("Slave (id: {}) not ready!", slave_id);
                nb_dropped +=1;
                continue;
            }

            log::debug!("Got commands {:?}", req);
            for cmd in req.commands {
                slave_id = cmd.id as u32;

                let mut target_pos = cmd.target_position;
                match cmd.published_timestamp{
                    Some(published_time) => {
                        let published_time = match SystemTime::try_from(published_time) {
                            Ok(systime) => systime,
                            Err(_) => {
                                log::warn!("Cannot parse the timestamp, discarding message!");
                                continue;
                            }
                        };
                        // check if the message is older than allowed time
                        if self.controller.check_if_too_old(published_time.elapsed().unwrap()) {
                            nb_dropped +=1;
                            continue;
                        }
                    }
                    None => {
                        log::warn!("No published timestamp, discarding message!");
                        continue;   
                    }
                }

                let no_axis = self.controller.get_orbita_type(slave_id) as usize;

                let mut set_compliant = cmd.compliancy;

                let mode_of_operation = cmd.mode_of_operation;
                if mode_of_operation != 0 {
                    self.controller.set_mode_of_operation(slave_id as u16, mode_of_operation as u8).unwrap_or_else(
                        |e| log::error!("Failed to set mode of operation for slave {}: {}", slave_id, e),
                    );
                }

                if target_pos.len() != 0 {
                    // set only last target command 
                    self.controller.set_target_position(
                        slave_id, 
                        target_pos[(target_pos.len()-no_axis)..].to_vec()
                    ).unwrap_or_else(|e| {
                        log::error!("Failed to set target position for slave {}: {}", slave_id, e);
                        set_compliant = Some(true); // disable the slave!
                    });
                }
                let velocity_limit = cmd.velocity_limit;
                if velocity_limit.len() != 0 {
                    // set only last target command 
                    self.controller.set_velocity_limit(
                        slave_id, 
                        velocity_limit[(velocity_limit.len()-no_axis)..].to_vec()
                    ).unwrap_or_else(|e| {
                        log::error!("Failed to set velocity limit for slave {}: {}", slave_id, e);
                        set_compliant = Some(true); // disable the slave!
                    });
                }
                let torque_limit = cmd.torque_limit;
                if torque_limit.len() != 0 {
                    // set only last target command 
                    self.controller.set_torque_limit(
                        slave_id, 
                        torque_limit[(torque_limit.len()-no_axis)..].to_vec()
                    ).unwrap_or_else(|e| {
                        log::error!("Failed to set torque limit for slave {}: {}", slave_id, e);
                        set_compliant = Some(true); // disable the slave!
                    });
                }

                let target_velocity = cmd.target_velocity;
                if target_velocity.len() != 0 {
                    // set only last target command 
                    self.controller.set_target_velocity(
                        slave_id, 
                        target_velocity[(target_velocity.len()-no_axis)..].to_vec()
                    ).unwrap_or_else(|e| {
                        log::error!("Failed to set target velocity for slave {}: {}", slave_id, e);
                        set_compliant = Some(true); // disable the slave!
                    });
                }
                let target_torque = cmd.target_torque;
                if target_torque.len() != 0 {
                    // set only last target command 
                    self.controller.set_target_torque(
                        slave_id, 
                        target_torque[(target_torque.len()-no_axis)..].to_vec()
                    ).unwrap_or_else(|e| {
                        log::error!("Failed to set target torque for slave {}: {}", slave_id, e);
                        set_compliant = Some(true); // disable the save!
                    });
                }

                // check if the slave is compliant
                match set_compliant { 
                    Some(true) => self.controller.set_torque(slave_id, false).unwrap_or_else(
                        |e| log::error!("Failed to set torque off for slave {}: {}", slave_id, e),
                    ),
                    Some(false) => self.controller.set_torque(slave_id, true).unwrap_or_else(
                        |e| log::error!("Failed to set torque on for slave {}: {}", slave_id, e),
                    ),
                    None => (),
                }
                
                // DEBUGGING
                // check if the time between commands is longest than 
                // the previous longest time, and save it for debugging
                let dt_command =  t_command.elapsed().as_secs_f32();
                if dt_command_max < dt_command {
                    dt_command_max = dt_command;
                }
                // timestamp of the new command
                t_command = tokio::time::Instant::now();
                nb_commands += 1;
            }
            // wait for the next cycle  
            // to make sure the commands are executed
            // self.controller.inner.wait_for_next_cycle();

            // DEBUGGING
            // display the status report to the user each 10s
            let dt_debug = t_debug.elapsed().as_secs_f32();
            if dt_debug > report_display_time {
                let f_commands = nb_commands as f32 / dt_debug ;
                let f_commands_dropped = nb_dropped as f32 / dt_debug;
                let dt_commands_ms = (dt_debug) / (nb_commands as f32) * 1000.0;
                let dt_command_max_ms = dt_command_max * 1000.0;
                log::info!("GRPC EtherCAT Slave {} | commands sent {:0.2} cmd/s, commands dropped {:0.2} req/s, command time: {:0.2} (max {:0.2}) ms", 
                            slave_id, 
                            f_commands, 
                            f_commands_dropped, 
                            dt_commands_ms, 
                            dt_command_max_ms);

                t_debug = tokio::time::Instant::now();
                nb_commands = 0;
                nb_dropped = 0;
                dt_command_max = 0.0;
                t_command = tokio::time::Instant::now();
                report_display_time = 30.0; // [secs] next report display 
            }
        }

        Ok(Response::new(()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<_> = env::args().collect();
    let filename = match args.len() {
        2 => &args[1],
        _ => {
            println!("usage: {} ESI-FILE", env!("CARGO_PKG_NAME"));
            return Ok(());
        }
    };

    let controller = PoulpeController::connect(filename)?;

    for slave_id in controller.get_slave_ids() {
        log::info!("Setup Slave {}...", slave_id);
        match controller.setup(slave_id) {
            Ok(_) => log::info!("Done!"),
            Err(e) => {
                log::error!("Failed to setup slave {}: {}", slave_id, e);
                Err(e)?;
            }
        }
    }

    log::info!("POULPE controller ready!");

    let addr = "[::]:50098".parse()?;
    let srv = PoulpeMultiplexerService {
        controller: Arc::new(controller),
    };

    Server::builder()
        .add_service(PoulpeMultiplexerServer::new(srv))
        .serve(addr)
        .await?;

    Ok(())
}


fn poule_empty_state() -> PoulpeState {
    PoulpeState {
        id: 0,
        mode_of_operation: 0,
        actual_position: vec![],
        actual_velocity: vec![],
        actual_torque: vec![],
        axis_sensors: vec![],
        axis_sensor_zeros: vec![],
        motor_temperatures: vec![],
        board_temperatures: vec![],
        requested_target_position: vec![],
        requested_velocity_limit: vec![],
        requested_torque_limit: vec![],
        state: 0,
        error_codes: vec![],
        compliant: false,
        published_timestamp: None,
    }
}