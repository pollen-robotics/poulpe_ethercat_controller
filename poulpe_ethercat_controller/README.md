# `poulpe_ethercat_controller` crate

This crate wraps the `ethercat_contoller` crate and provides a more actuator oriented interface to communicate to the poulpe boards. 

## Safety features


This crate in addition to all the EtherCAT specific safety features implemented in the `ethercat_controller` crate, also implements the actuators safety features such as:
- Verification of the ethercat network topology on startup (has to correspond to the network configuration yaml file).
    - Fails if all the slaves are not connected 
    - Fails if there are more slaves connected than specified in the network configuration file, can be disabled with the `allow_partial_network` feature.
    - Fails if the slaves are not at expected addresses.
    - Fails if the slave type is not the expected one, this check is only performed if the feature `verify_orbita_type` is enabled.
    - Fails if any of the slabves is in the error state, can be disabled with the `allow_fault_on_slave` feature.
- Turn off the actuators on the controller starup, making sure that the actuators are in the `SwitchedOn` state (no voltage applied to the motors).
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
`allow_partial_network` | Allows to use only a part of the slaves connected | yes
`allow_fault_on_slave` | Does not stop the operation if one slave is in the fault state | yes
`turn_off_slaves_setup` | Turn off the slaves on startup | yes
`safe_turn_on` | Set the target position to the current position on every turn on | yes
`switchon_on_turnon` | Switch on the actuator on turn on (if it is in the `SwitchedOnDisabled` state) | no


See the and configure the features in the [Cargo.toml](Cargo.toml) file.