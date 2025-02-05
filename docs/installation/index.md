---
title: Installation and configuration
layout: default
position: 2
has_children: true
---

# Installation and configuration

The `poulpe_ethercat_controller` crate is intended to be used with the poulpe boards that are connected to the network. The crate can be used to communicate with the poulpe boards, read and write the SDO and PDO objects, and update the firmware over the EtherCAT network.

## Downoload the code

Now that you have the ethercat master running and the poulpe board configured, you can run the code.

- Clone the repo
```shell
git clone git@github.com:pollen-robotics/poulpe_ethercat_controller.git
```

## Building the code

The crate is written in Rust and is communicating difectly wit the EtherCAT IgH Master. It uses the rust wrapper crate `ethercat-rs` and builds on top of it to provide a more user friendly interface to the user. The endpoint interface is a GRPC server and client interface that can be accessed by multiple clients at the same time, either in rust or python.

So the main dependencies are:
- Rust - [installation guide](https://www.rust-lang.org/tools/install)
- EtherCAT IgH Master (for the EtherCAT communication) - [installation guide](installation_ethercat)

Once you have all the dependencies installed you can build the code with:
```shell
cargo build --release
```

## Configuration

Once when you have everything installed you and before you can run the code you need to configure the poulpe boards on the network if they are not already configured. The configuration is done with the EtherCAT network configuration files. The configuration files are located in the `config` directory. 

Here is the [guide to configure the poulpe boards](configure_poulpe) on the network. 

