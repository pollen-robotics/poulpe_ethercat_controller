---
title: Installation and configuration
layout: default
nav_order: 2
has_children: true
---

# Installation and configuration

The crate is written in Rust and is communicating difectly wit the **EtherCAT IgH** Master. It uses the rust wrapper crate `ethercat-rs` and builds on top of it to provide a more user friendly interface to the user. 

## Downoload the code

You can obtain the code by cloning the repository.

- Clone the repo
```shell
git clone git@github.com:pollen-robotics/poulpe_ethercat_controller.git
```

Check out the branches that you need, depending on the poulpe firmware version that you are using. 

`firmware_poulpe` version | `poulpe_etehract_controller` version
--- | ---
v0.9.x | 0.9.x
v1.0.x | 1.0.x or higher
v1.5.x | 1.5.x 

For example if you are using the v1.5.x firmware version you should check out the 1.5.x branch:
```shell
git checkout 1.5.x
```



## Building the code

The crate is written in Rust and uses the `cargo` build system. All the dependencies are managed by the `cargo` build system, except for the EtherCAT IgH Master.

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


## Running the code

Once you have the ethercat master running and you connected your poulpe board to the network, you can run the code. See the [Running the code](../examples) docs for more info.