use clap::Parser;

use owo_colors::{colors::*, OwoColorize};
use std::{error::Error, thread, time::Duration};

use poulpe_ethercat_grpc::client::PoulpeIdClient;
use poulpe_ethercat_grpc::server::launch_server;
use poulpe_ethercat_grpc::PoulpeRemoteClient;

use poulpe_ethercat_controller::state_machine::{
    parse_homing_error_flags, parse_motor_error_flags, parse_state_from_status_word,
    parse_status_word, CiA402State, HomingErrorFlag,
};

/// Checking all devices on the bus
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// ethercat config
    #[arg(short, long)]
    configfile: String,

    #[arg(short, long)]
    start_server: bool,
}

// takes the salve id as argument
// and moves the motor in a sinusoidal motion
fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let args = Args::parse();
    if args.start_server {
        println!("{}", "Starting the server".bold().underline());
        // run in a thread, do not block main thread
        thread::spawn(move || {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .unwrap()
                .block_on(launch_server(&args.configfile))
                .unwrap();
        });
        thread::sleep(Duration::from_secs(2));
    }

    let idclient = PoulpeIdClient::new("http://127.0.0.1:50098".parse()?);
    match idclient.get_slaves() {
        Ok((ids, names)) => {
            println!(
                "{} {} {:?} {} {:?}",
                "Bus scanned!".bold(),
                "\nDetected ids:".bold(),
                ids,
                "names:".bold(),
                names
            );
            // log::info!("Config file: {}", args.configfile);

            let client = PoulpeRemoteClient::connect(
                "http://127.0.0.1:50098".parse()?,
                ids.clone(),
                Duration::from_secs_f32(0.001),
            )?;
            println!("{}\n", "Checking slaves:".bold());
            let mut orbita_with_problems: Vec<String> = Vec::new();
            for it in ids.iter().zip(names.iter()) {
                let mut err: bool = false;
                let (id, name) = it;
                println!(
                    "{}",
                    "                                                                 ".underline()
                );
                println!("{} {} {} {}", "Slave id:".bold(), id, "name:".bold(), name);

                thread::sleep(Duration::from_secs(1));

                let state = client.get_cia402_state(*id).unwrap();
                let cia_state: CiA402State = parse_state_from_status_word(state as u16);
                let status_bits = parse_status_word(state as u16);
                print!("\t{}", "Board State: ".bold());
                match cia_state {
                    CiA402State::NotReadyToSwitchOn => println!("{:?}", cia_state.yellow().bold()),
                    CiA402State::FaultReactionActive | CiA402State::Fault => {
                        println!("{:?}", cia_state.red().bold())
                    }
                    _ => println!("{:?}", cia_state.green().bold()),
                }
                println!("\t{}  {:?}", "Status bits".bold(), status_bits);

                let error_codes = client.get_error_codes(*id).unwrap();
                let homing_error_flags =
                    parse_homing_error_flags((error_codes[0] as u16).to_le_bytes());
                print!("\t{}", "Homing Error flags: ".bold(),);
                if !homing_error_flags.is_empty() {
                    println!("{:?}", homing_error_flags.red().bold());
                    err = true;
                } else {
                    println!("{}", "No error".green().bold());
                }
                for (i, e) in error_codes.iter().enumerate().skip(1) {
                    let motor_error = parse_motor_error_flags((*e as u16).to_le_bytes());
                    print!("\t{} {} {}", "Motor ".bold(), i, " | Error flags: ".bold());
                    if !motor_error.is_empty() {
                        println!("{:?}", motor_error.red().bold());
                        err = true;
                    } else {
                        println!("{}", "No error".green().bold());
                    }
                }
                println!();
                if err {
                    orbita_with_problems.push(name.into());
                }
            }
            if orbita_with_problems.is_empty() {
                println!("{}", "No problem detected".bold().green());
            } else {
                println!(
                    "{} {} {:?}",
                    "Problem detected!\n".bold().red().blink_fast(),
                    "\tPlease check: ".red().bold(),
                    orbita_with_problems.bold()
                )
            }
        }
        Err(e) => {
            log::error!("Unable to scan bus! {:?}", e);
        }
    };

    Ok(())
}
