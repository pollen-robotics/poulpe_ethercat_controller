---
title: Safety features
layout: default
---


# Safety features

Each layer of the code has its own safety features. The `ethercat_controller` deals with the EtherCAT communication safety features (see more in the [ethercat_controller docs](../software/ethercat_controller#safety-features)). The `poulpe_ethercat_controller` crate has its own safety features that are specific to the poulpe boards (see more in the [poulpe_ethercat_controller docs](../software/poulpe_ethercat_controller#safety-features)). The `poulpe_ethercat_grpc` crate has its own safety features that are specific to the GRPC communication (see more in the [poulpe_ethercat_grpc docs](../software/poulpe_ethercat_grpc#safety-features)).

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


If any of the above safety features fails, the master will send an emergency stop signal (CiA402 QuickStop) to all the boards and stop the operation. The boards will go into the shut-down state. 


## Emergency stop

Additionally, anytime the ethercat master stops the operation, either in [standalone](../examples/standalone) or in the [GRPC server](../examples/grpc) mode, the poulpe boards will stop their operation and go into the shut-down state. 

However, if the GRPC server is up and running, there is another way to send the emergency stop command to the boards without killing the server, using the emergency stop script. This usage is useful in order to enable providing the applications built on top of the GRPC server with the state data of the boards even after the emergency stop signal is sent. The code contains the precompiled script that sends the emergency stop (CiA402 QuickStop command) signal to all the boards connected to the ethercat network.

To run the script, navigate to the main directory and run the following command:

```shell
sh emergency_stop_all.sh
```