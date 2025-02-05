# Poulpe ethercat controller

[![Build Status]][actions]

[Build Status]: https://img.shields.io/github/actions/workflow/status/pollen-robotics/poulpe_ethercat_controller/rust.yml?branch=develop
[actions]: https://github.com/pollen-robotics/poulpe_ethercat_controller/actions?query=branch%3Adevelop


This is the code that manages the communication between the orbita2d and orbita3d actuators and the ethercat master. The code is written in rust. It is intended to communicate with poulpe boards running the [firmware_Poulpe](https://github.com/pollen-robotics/firmware_Poulpe).

The full stack looks something like this:

<img src="docs/images/grpc_full_stack.png" width="900">

`ethercat_controller` creates the direct connection to the EtherCAT master deamon (which communicates with the poulpe boards). `poulpe_ethercat_controller` provides the abstraction layer for the poulpe boards around the `ethercat_controller`. Finally, `poulpe_ethercat_grpc` creates the `server` that can be accessed by multiple `client` instances.

Fin out more in our docs: [docs](docs/index.md)

## Installation

Se more complete installation guide in the [docs](docs/index.md)

### Prerequisites

For using this code, you need to have the following installed: [Ethercat master](https://gitlab.com/etherlab.org/ethercat)

The you can start the master with:
```shell
sudo ethercatctl start
```
And see the connected slaves with:
```shell
ethercat slaves
```

### Building

Clone the repo
```shell
git clone git@github.com:pollen-robotics/poulpe_ethercat_controller.git
```

Make sure to have rust installed: [Rust](https://www.rust-lang.org/tools/install) as well as the Ethercat master.


```shell
cargo build --release
```

### Running

Make sure you have your EteherCAT master running and the poulpe boards connected.

To run the server, you need to have the ethercat master running.

```shell
RUST_LOG=info cargo run --release config/ethercat.yaml
```

<details markdown="1"><summary>Example output with only one slave connected (NeckOrbita3d):</summary>

```shell
$ RUST_LOG=info cargo run --release config/ethercat.yaml

[2024-12-03T07:58:37Z INFO  ethercat_controller::ethercat_controller] Found 1 slaves
[2024-12-03T07:58:37Z INFO  ethercat_controller::ethercat_controller] Slave "NeckOrbita3d" at position 0
[2024-12-03T07:58:37Z INFO  server] Setup Slave 0...
[2024-12-03T07:58:37Z INFO  ethercat_controller::ethercat_controller] Master and all slaves operational!
[2024-12-03T07:58:37Z INFO  poulpe_ethercat_controller] Slave 0, inital state: SwitchOnDisabled
[2024-12-03T07:58:37Z INFO  poulpe_ethercat_controller] Slave 0, setup done! Current state: SwitchedOn
[2024-12-03T07:58:37Z INFO  server] Done!
[2024-12-03T07:58:37Z INFO  server] POULPE controller ready!
[2024-12-03T07:58:47Z INFO  ethercat_controller::ethercat_controller] EtherCAT loop: 913.37 Hz
...
```
</details>


The server is now running and you can connect to it using the clients. For example to make a sinusoidal movement with the NeckOrbita3d, you can run the following command:

```shell
RUST_LOG=info cargo run --release --examples client_sinus 0 # takes the slave id or name as argument
```


## Support

This project adheres to the Contributor [code of conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to [contact@pollen-robotics.com](mailto:contact@pollen-robotics.com).

Visit [pollen-robotics.com](https://pollen-robotics.com) to learn more or join our [Dicord community](https://discord.gg/vnYD6GAqJR) if you have any questions or want to share your ideas.
Follow [@PollenRobotics](https://twitter.com/pollenrobotics) on Twitter for important announcements.
