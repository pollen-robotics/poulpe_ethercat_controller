---
title: Configure Poulpes
parent: Installation and configuration
layout: default
---

# Configuring the Poulpe board for the EtherCAT network

Once you have the ethercat master running and you connected your poulpe board to the network, you need to configure the poulpe board to work with the network.

There are two main steps to prepare the poulpe board for the ethercat network:
- Make sure the poulpe is running the appropriate version of the firmware
- Make sure the EEPROM of the LAN9252 chip on the poulpe board is flashed with the appropriate configuration file.

## Firmware version

- Make sure that the poulpe board is running the appropriate version of the firmware.

`firmware_poulpe` version | `poulpe_etehract_controller` version
--- | ---
v0.9.x | v0.9.x
v1.0.x | v1.0.x or higher
v1.5.x | v1.5.x 

## LAN9252 configuration
- Make sure that the poulpe board is configured properly for the ethercat network.
    - The EEPROM of the LAN9252 chip on the poulpe board needs to be flashed with the appropriate configuration file.

- If the board is not configured, once connected to the ethercat network, you it will not display its proper name when you run `ethercat slave`

```sh
$ ethercat slave
0  0:0  OP  +  00000000:00000000
```

- This means that the EEPROM is not configured. To configure it youn need to flush the binary config file to the EEPROM using the `ethercat sii_write` command.
    - ex. `ethercat sii_write -p0 Orbita2d.bin` (may need sudo)
        - If there is an error in the transfer try disconnecting and reconnecting the the ethernet cable.
        - **Make sure to restart the board after this (diconnect the power)**
    - The configuration files are located in the `config/esi` directory.
    - See the [config/esi/README.md](config/esi/README.md) for more info.

- Then you should have the proper name of the device. For example:
```sh
$ ethercat slave
0  0:0  OP  +  Orbita2d
```

- You can also check the available PDO mappings with `ethercat pdos`.

<details markdown="1"><summary>Example for version v0.9.x</summary>

```sh
$ ethercat pdos

SM0: PhysAddr 0x1000, DefaultSize    0, ControlRegister 0x64, Enable 1
  RxPDO 0x1600 "MotorIn"
    PDO entry 0x0010:01,  8 bit, "torque_state"
    PDO entry 0x0011:01, 32 bit, "target"
    PDO entry 0x0011:02, 32 bit, "velocity_limit"
    PDO entry 0x0011:03, 32 bit, "torque_limit"
    PDO entry 0x0012:01, 32 bit, "target"
    PDO entry 0x0012:02, 32 bit, "velocity_limit"
    PDO entry 0x0012:03, 32 bit, "torque_limit"
SM1: PhysAddr 0x1200, DefaultSize    0, ControlRegister 0x20, Enable 1
  TxPDO 0x1a00 "Orbita"
    PDO entry 0x0020:00,  8 bit, "state"
    PDO entry 0x0020:01,  8 bit, "type"
SM2: PhysAddr 0x1300, DefaultSize    0, ControlRegister 0x22, Enable 1
  TxPDO 0x1a01 "MotorOut"
    PDO entry 0x0030:00,  8 bit, "torque_enabled"
    PDO entry 0x0031:01, 32 bit, "position"
    PDO entry 0x0031:02, 32 bit, "velocity"
    PDO entry 0x0031:03, 32 bit, "torque"
    PDO entry 0x0031:04, 32 bit, "axis_sensor"
    PDO entry 0x0032:01, 32 bit, "position"
    PDO entry 0x0032:02, 32 bit, "velocity"
    PDO entry 0x0032:03, 32 bit, "torque"
    PDO entry 0x0032:04, 32 bit, "axis_sensor
```
</details>

<details markdown="1"><summary>Example for version v1.0.x</summary>

```sh
$ ethercat pdos
SM0: PhysAddr 0x1000, DefaultSize    0, ControlRegister 0x64, Enable 1
  RxPDO 0x1600 "OrbitaIn"
    PDO entry 0x6041:00, 16 bit, "controlword"
    PDO entry 0x6060:00,  8 bit, "mode_of_operation"
    PDO entry 0x607a:01, 32 bit, "target_position"
    PDO entry 0x607a:02, 32 bit, "target_position"
    PDO entry 0x607a:03, 32 bit, "target_position"
    PDO entry 0x60ff:01, 32 bit, "target_velocity"
    PDO entry 0x60ff:02, 32 bit, "target_velocity"
    PDO entry 0x60ff:03, 32 bit, "target_velocity"
    PDO entry 0x607f:01, 32 bit, "velocity_limit"
    PDO entry 0x607f:02, 32 bit, "velocity_limit"
    PDO entry 0x607f:03, 32 bit, "velocity_limit"
    PDO entry 0x6071:01, 32 bit, "target_torque"
    PDO entry 0x6071:02, 32 bit, "target_torque"
    PDO entry 0x6071:03, 32 bit, "target_torque"
    PDO entry 0x6072:01, 32 bit, "torque_limit"
    PDO entry 0x6072:02, 32 bit, "torque_limit"
    PDO entry 0x6072:03, 32 bit, "torque_limit"
SM1: PhysAddr 0x1200, DefaultSize    0, ControlRegister 0x22, Enable 1
  TxPDO 0x1700 "OrbitaState"
    PDO entry 0x603f:00, 16 bit, "error_code"
    PDO entry 0x603f:01, 16 bit, "error_code"
    PDO entry 0x603f:02, 16 bit, "error_code"
    PDO entry 0x603f:03, 16 bit, "error_code"
    PDO entry 0x6402:00,  8 bit, "actuator_type"
    PDO entry 0x607c:01, 32 bit, "axis_position_zero_offset"
    PDO entry 0x607c:02, 32 bit, "axis_position_zero_offset"
    PDO entry 0x607c:03, 32 bit, "axis_position_zero_offset"
    PDO entry 0x6500:01, 32 bit, "board_temperatures"
    PDO entry 0x6500:02, 32 bit, "board_temperatures"
    PDO entry 0x6500:03, 32 bit, "board_temperatures"
    PDO entry 0x6501:01, 32 bit, "motor_temperatures"
    PDO entry 0x6501:02, 32 bit, "motor_temperatures"
    PDO entry 0x6501:03, 32 bit, "motor_temperatures"
SM2: PhysAddr 0x1300, DefaultSize    0, ControlRegister 0x20, Enable 1
  TxPDO 0x1800 "OrbitaOut"
    PDO entry 0x6040:00, 16 bit, "statusword"
    PDO entry 0x6061:00,  8 bit, "mode_of_operation_display"
    PDO entry 0x6064:01, 32 bit, "actual_position"
    PDO entry 0x6064:02, 32 bit, "actual_position"
    PDO entry 0x6064:03, 32 bit, "actual_position"
    PDO entry 0x606c:01, 32 bit, "actual_velocity"
    PDO entry 0x606c:02, 32 bit, "actual_velocity"
    PDO entry 0x606c:03, 32 bit, "actual_velocity"
    PDO entry 0x6077:01, 32 bit, "actual_torque"
    PDO entry 0x6077:02, 32 bit, "actual_torque"
    PDO entry 0x6077:03, 32 bit, "actual_torque"
    PDO entry 0x6063:01, 32 bit, "actual_axis_position"
    PDO entry 0x6063:02, 32 bit, "actual_axis_position"
    PDO entry 0x6063:03, 32 bit, "actual_axis_position"
```

</details>

<details markdown="1"><summary>Example for version v1.0.x</summary>

```shell
$ ethercat pdos
SM0: PhysAddr 0x1000, DefaultSize  128, ControlRegister 0x26, Enable 1
SM1: PhysAddr 0x1180, DefaultSize  128, ControlRegister 0x22, Enable 1
SM2: PhysAddr 0x1300, DefaultSize    0, ControlRegister 0x64, Enable 1
  RxPDO 0x1600 "OrbitaIn"
    PDO entry 0x6041:00, 16 bit, "controlword"
    PDO entry 0x6060:00,  8 bit, "mode_of_operation"
    PDO entry 0x607a:01, 32 bit, "target_position"
    PDO entry 0x607a:02, 32 bit, "target_position"
    PDO entry 0x607a:03, 32 bit, "target_position"
    PDO entry 0x60ff:01, 32 bit, "target_velocity"
    PDO entry 0x60ff:02, 32 bit, "target_velocity"
    PDO entry 0x60ff:03, 32 bit, "target_velocity"
    PDO entry 0x607f:01, 32 bit, "velocity_limit"
    PDO entry 0x607f:02, 32 bit, "velocity_limit"
    PDO entry 0x607f:03, 32 bit, "velocity_limit"
    PDO entry 0x6071:01, 32 bit, "target_torque"
    PDO entry 0x6071:02, 32 bit, "target_torque"
    PDO entry 0x6071:03, 32 bit, "target_torque"
    PDO entry 0x6072:01, 32 bit, "torque_limit"
    PDO entry 0x6072:02, 32 bit, "torque_limit"
    PDO entry 0x6072:03, 32 bit, "torque_limit"
SM3: PhysAddr 0x1400, DefaultSize    0, ControlRegister 0x20, Enable 1
  TxPDO 0x1700 "OrbitaOut"
    PDO entry 0x6040:00, 16 bit, "statusword"
    PDO entry 0x6061:00,  8 bit, "mode_of_operation_display"
    PDO entry 0x6064:01, 32 bit, "actual_position"
    PDO entry 0x6064:02, 32 bit, "actual_position"
    PDO entry 0x6064:03, 32 bit, "actual_position"
    PDO entry 0x606c:01, 32 bit, "actual_velocity"
    PDO entry 0x606c:02, 32 bit, "actual_velocity"
    PDO entry 0x606c:03, 32 bit, "actual_velocity"
    PDO entry 0x6077:01, 32 bit, "actual_torque"
    PDO entry 0x6077:02, 32 bit, "actual_torque"
    PDO entry 0x6077:03, 32 bit, "actual_torque"
    PDO entry 0x6063:01, 32 bit, "actual_axis_position"
    PDO entry 0x6063:02, 32 bit, "actual_axis_position"
    PDO entry 0x6063:03, 32 bit, "actual_axis_position"
  TxPDO 0x1800 "OrbitaState"
    PDO entry 0x603f:00, 16 bit, "error_code"
    PDO entry 0x603f:01, 16 bit, "error_code"
    PDO entry 0x603f:02, 16 bit, "error_code"
    PDO entry 0x603f:03, 16 bit, "error_code"
    PDO entry 0x6402:00,  8 bit, "actuator_type"
    PDO entry 0x607c:01, 32 bit, "axis_position_zero_offset"
    PDO entry 0x607c:02, 32 bit, "axis_position_zero_offset"
    PDO entry 0x607c:03, 32 bit, "axis_position_zero_offset"
    PDO entry 0x6500:01, 32 bit, "board_temperatures"
    PDO entry 0x6500:02, 32 bit, "board_temperatures"
    PDO entry 0x6500:03, 32 bit, "board_temperatures"
    PDO entry 0x6501:01, 32 bit, "motor_temperatures"
    PDO entry 0x6501:02, 32 bit, "motor_temperatures"
    PDO entry 0x6501:03, 32 bit, "motor_temperatures"
```

</details>