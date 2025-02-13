---
title: Installation and configuration
layout: default
nav_order: 2
has_children: true
---

# Installation and configuration

The crate is written in Rust and is communicating difectly wit the **EtherCAT IgH** Master. It uses the rust wrapper crate `ethercat-rs` and builds on top of it to provide a more user friendly interface to the user. 

### Pre-requisites

Before you can build and run the code you need to have the following installed on your system:

- Rust - [installation guide](https://www.rust-lang.org/tools/install)
- EtherCAT IgH Master (for the EtherCAT communication) - [installation guide](installation_ethercat)

###  Code version

This crate is designed to work with the poulpe firmware version, so make sure to check the version of the firmware that you are using and check out the corresponding branch of the code.

`firmware_poulpe` version | `poulpe_etehract_controller` version
--- | ---
v0.9.x | 0.9.x
v1.0.x | 1.0.x or higher
v1.5.x | 1.5.x 


### Poulpe board EtherCAT configuration

Once when you have everything installed you and before you can run the code you need to configure the poulpe boards on the network if they are not already configured. The configuration is done with the EtherCAT network configuration files. The configuration files are located in the `config` directory. 

Here is the [guide to configure the poulpe boards](configure_poulpe) on the network. 

## Importing the crate

The crate is designed to be used as a library in your project. You can include it in your `Cargo.toml` file as a dependency:

```toml
....
[dependencies]
...
poulpe_ethercat_controller = { git = "https://github.com/pollen-robotics/poulpe_ethercat_controller.git", branch = "1.5.x" }
```

You can also include only sub-crates that you need. For example, if you only need the GRPC client

```toml
....
[dependencies]
...
poulpe_ethercat_grpc = { git = "https://github.com/pollen-robotics/poulpe_ethercat_controller.git", branch = "1.5.x" }
```

## Python-only installation

If you are using the python-only version of the code you can install the code directly from the repository. 
Make sure to use the tag corresponding to the version of the firmware that you are using. For example, if we use the 1.5.4 tag: (see the [tags](https://github.com/pollen-robotics/poulpe_ethercat_controller/tags))

```shell
pip install git+https://github.com/pollen-robotics/poulpe_ethercat_controller.git@1.5.4#subdirectory=python_client
```

NOTE:
This install procedure is available from the version 1.5.4 of the `poulpe_ethercat_controller`.

Once this is installed you can use the python client to communicate with the poulpe board. 

```python
import python_client
```

## Installing from source

You can obtain the code by cloning the repository.

- Clone the repo
```shell
git clone git@github.com:pollen-robotics/poulpe_ethercat_controller.git
```

- Check out the branches that you need, depending on the poulpe firmware version that you are using.  For example if you are using the v1.5.x firmware version you should check out the 1.5.x branch:
```shell
git checkout 1.5.x
```

- Once you have all the dependencies installed you can build the code with:
```shell
cargo build --release
```

See how to [install the python bindings](python_client) from the source code.




## Running the code

Once you have the ethercat master running and you connected your poulpe board to the network, you can run the code. See the [Running the code](../examples) docs for more info.