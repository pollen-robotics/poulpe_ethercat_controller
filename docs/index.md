---
title: Home
layout: default
nav_order: 1
---
# Poulpe ehtercat controller

This is the code that manages the communication between the orbita2d and orbita3d actuators and the ethercat master. The code is written in rust. It is intended to communicate with poulpe boards running the [firmware_Poulpe](https://github.com/pollen-robotics/firmware_Poulpe).



## Safety features

Each layer of the code has its own safety features. The `ethercat_controller` deals with the EtherCAT communication safety features (see more in the [ethercat_controller/README.md](ethercat_controller/README.md#main-features)). The `poulpe_ethercat_controller` crate has its own safety features that are specific to the poulpe boards (see more in the [poulpe_ethercat_controller/README.md](poulpe_ethercat_controller/README.md#safety-features)). The `poulpe_ethercat_grpc` crate has its own safety features that are specific to the GRPC communication (see more in the [poulpe_ethercat_grpc/README.md](poulpe_ethercat_grpc/README.md#safety-features)).

`ethercat_controller` crate has the following safety features:
- At the statup
    - Checks if the master and all the slaves are oprational
    - Checks if all the slaves are configured properly
- During the operation
    - Checks if the master and all the slaves are oprational
    - Checks if all the slaves are connected to the master
    - Checks if new slaves are connected to the master

`poulpe_ethercat_controller` crate has the following safety features:
- At the statup
    - Checks if ethercat network is operational and the topology is correct
    - Checks if all the boards are in the correct state
- During the operation
    - Checks if the boards are in the correct state and only allows turning them on if they are in the correct state

`poulpe_ethercat_grpc` crate has the following safety features:
- Real-time communication
    - All server and client messages are time stamped to ensure that the communication is real-time
    - The server discards all the client messages that are too old
    - The client that receives the messages that are too old will not process them and consider that the server is down
- Safety features
    - The server checks if the boards are in the fault state and if any of them is it sends the emergency stop signal to all the boards
    - The server continues the operation, reading the baoards states but not sending any commands to the boards

## Install and build the `poulpe_ethercat_controller` code

Now that you have the ethercat master running and the poulpe board configured, you can run the code.

- Clone the repo
```shell
git clone git@github.com:pollen-robotics/poulpe_ethercat_controller.git
```

- Build the code
```shell
cargo build --release
```
If the build passes that means that the code is working properly.


## Support

This project adheres to the Contributor [code of conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to [contact@pollen-robotics.com](mailto:contact@pollen-robotics.com).

Visit [pollen-robotics.com](https://pollen-robotics.com) to learn more or join our [Dicord community](https://discord.gg/vnYD6GAqJR) if you have any questions or want to share your ideas.
Follow [@PollenRobotics](https://twitter.com/pollenrobotics) on Twitter for important announcements.
