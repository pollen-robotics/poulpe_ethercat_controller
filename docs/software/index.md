--- 
title: Crates
layout: default
nav_order: 3
has_children: true
---

# Software architecture


`poulpe_ethercat_controller` crate is intended to be used with the poulpe boards that are connected to the network. The crate can be used to communicate with the poulpe boards, read and write the SDO and PDO objects, and update the firmware over the EtherCAT network. The endpoint interface is a GRPC server and client interface that can be accessed by multiple clients at the same time, either in rust or python.

### Crate structure

- `ethercat_controller`: This is the main crate that does the heavy lifting of the communication with the ethercat master.
    - It is a wrapper around the `ethercat-rs` crate. This crate enables to create the ethercat master form an ESI xml file.
    - See more in [ethercat_controller](ethercat_controller)
- `poulpe_ethercat_controller`: This is an abstraction layer on top of the `ethercat_controller` crate. It provides a more user friendly interface to the user with specific functions for poulpe boards.
    - See more in the [poulpe_ethercat_controller](poulpe_ethercat_controller)
- `poulpe_ethercat_grpc`: This crate uses the `poulpe_ethercat_controller` to allow for reading assynchronously from multiple poulpe boards connected to the same ethercat master. It is based on the `grpc` protocol. It allows for creating a single server that can be accessed by multiple clients.
    - See more in the [poulpe_ethercat_grpc](poulpe_ethercat_grpc)
- `python_client`: This is a python wrapper of the `poulpe_ethercat_grpc` crate's client side. It allows for reading from multiple poulpe boards connected to the same ethercat master from python and in that way enables quick prototyping.
    - See more in the [python_client](python_client)
- `config`: This is a directory that contains the configuration files for the poulpe boards. It contains the eeprom configuration files for the LN9252 chip on the poulpe boards as well as the EtherCAT networks slave configuration yaml files that are used to create the ethercat master.
    - See more in the [config](config)
The full stack looks something like this:

<img src="../images/grpc_full_stack.png" width="900">

`ethercat_controller` creates the direct connection to the EtherCAT master deamon (which communicates with the poulpe boards). `poulpe_ethercat_controller` provides the abstraction layer for the poulpe boards around the `ethercat_controller`. Finally, `poulpe_ethercat_grpc` creates the `server` that can be accessed by multiple `client` instances.