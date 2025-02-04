# `poulpe_ethercat_controller` crate

This crate wraps the `ethercat_contoller` crate and provides a more actuator oriented interface to communicate to the poulpe boards. 

## Safety features


This crate in addition to all the EtherCAT specific safety features implemented in the `ethercat_controller` crate. The crate provides the `PoulpeEthercatController` struct which creates and communicates with the poulpe boards through the `ethercat_controller` crate. 
Upon creation the `PoulpeEthercatController` it will automatically scan the network and determine the names and ids of all the poulpe boards in the network. It will also do some checks to make sure that the poulpe boards are correctly configured:
- Constructing the map of the ethercat network topology on startup will do the following checks:
    - Fails if there are multiple poulpe with the same name in the network 
      - This should not happen unless a poulpe board is not (well) configured 
    - Fails if the slave type is not the expected one, this check is only performed if the feature `verify_orbita_type` is enabled. 
      - This check is used to make sure that the firmware is well configured and the slave is the expected one. (not too important but a good practice)

The configuration of slaves is done in a lazy way, meaning that the slaves are not configured until the first time the funciton `setup` is called. This is done to avoid configuring the slaves that are not used in the application.
- On every slave `setup` (after the statup) the controller will:
    - Verify that the slave is in the ethercat network and has a good name before connecting. 
    - Verify that the slave name in the network is the same as the name in the map of the network topology created at startup.
    - Making sure that the actuators are in the `SwitchedOn` state (no voltage applied to the motors).
        - If the actuators are not in the `OperationEnabled` state, the controller will perform the emergency stop on the actuator. And then proceed to turning it on again to the `SwitchedOn` state.
        - This behavior is enabled with the `turn_off_slaves_setup` feature (enabled by default).

Additionally on every `turn_on` command for the actuators, which can be dangerous if the actuators are not properly initialized, the controller execution will:

- Make sure that the actuators target position is the same as the current position. 
    - This behavior is enabled with the `safe_turn_on` feature.
- Fail if any of the actuators is in the error state, can be disabled with the `allow_fault_on_slave` feature.
- Fail if the actuator is not in the good state. In order to be able to go to the `OperationEnabled` state, the actuator has to be in the `SwitchedOn` state. 
    - If the actuator is not in the `SwitchedOnDisabled` state, the controller will fail (for example after a power cycle or a emergency stop). 
    - If the feature `switchon_on_turnon` is enabled, the controller will try to switch on the actuator if it is in the `SwitchedOnDisabled` state.

## List of features

feature | description | enabled by default
--- | --- | ---
`verify_orbita_type` | Verify the type of the slave on startup | yes
`verify_network_on_slave_setup` | Verify that the slave is in ethercat network and has a good name before connecting | yes
`allow_fault_on_slave` | Does not stop the operation if one slave is in the fault state | yes
`turn_off_slaves_setup` | Turn off the slaves on startup | yes
`safe_turn_on` | Set the target position to the current position on every turn on | yes
`switchon_on_turnon` | Switch on the actuator on turn on (if it is in the `SwitchedOnDisabled` state) | no


See the and configure the features in the [Cargo.toml](Cargo.toml) file.