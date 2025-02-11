use std::env;

use poulpe_ethercat_grpc::server::launch_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<_> = env::args().collect();
    let filename = match args.len() {
        2 => &args[1],
        _ => {
            log::warn!("No configuration file provided, in the arguments, using default config/ethercat.yaml!");
            "config/ethercat.yaml"
        }
    };

    // launch the server
    launch_server(filename).await?;

    Ok(())
}
