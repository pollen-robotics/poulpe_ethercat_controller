# Reacy2 ESI files - firmware 1.0

This directory has all the necessary EtherCAT slave information (ESI) files for the orbita joints of the reachy2 roobt. 
It contrains
- XML ESI files `*.xml`
- compiles ESI file `*.bin`
- complete robot XML ESI file generator `generate_robot_esi_files.py`
- bash compilation script for the XML files `compile_esi.sh`

## Generating the ESI files 

This step requires installing: 
- `pyesi` : https://github.com/pollen-robotics/pyesi
- `siitool` : https://github.com/synapticon/siitool

To regeneate the `xml` files  (requires `pyesi`)
```bash
python3 -m generate_robot_esi_files
```

to generate the bin files (requires `siitool`)
```bash
sh compile_esi.sh
```

## Flushing the compiled ESI files to poulpe boards

This step requires having and running the ethercat IgH master on your PC
- `ethercat-master` - https://gitlab.com/etherlab.org/ethercat

Once you have the binary file, you can use the `ethercat` tool to flash the EEPROM. 

For example to flash the neck orbita3d witch is placed on the slave address `0` we do this:

```bash
ethercat sii_write -p0 NeckOrbita3d.bin # may need sudo
```
For right wrist at the address `4`
```bash
ethercat sii_write -p4 RightWristOrbita3d.bin # may need sudo
```

> If there is an error in the transfer try disconnecting and reconnecting the the ethernet cable.


**Make sure to restart the boarda after this (diconnect the power)**

Make sure to put the proper slave index `p0` is for slate 0, `p1` for slave 1, etc.
Also make sure to use the proper binary file (ex. `LeftShoulderOrbita3d.bin` for left shoulder).

# 
# PDOs structure

The full CiA402 design specification can be found here: [dsp402.pdf](../../docs/images/dsp402.pdf)

Here is a summary of the commonly used PDO structures:
- RxPDOs: [some nice docs](https://doc.synapticon.com/node/sw5.1/object_dict/pdo/rxpdo.html)
- TxPDOs: [some nice docs](https://doc.synapticon.com/node/sw5.1/object_dict/pdo/txpdo.html?tocpath=Software%20Reference%205.1%7CProcess%20Data%20Objects%20(PDO)%7C_____2)

Here is the chosen structure of the PDOs. They now follow relatively well the CiA4 standard. 
The Indexes correspond to the indexes in the standard and the sub-indexes correspond to the axis of the actuaror 2 axis for orbita2d and 3 axis for orbita 3d.

- `OrbitaIn` (RxPdo) - Master to Slave
- `OrbitaState` (TxPdo) - Slave to Master
- `OrbitaOut` (TxPdo) - Slave to Master



| Attribute | `OrbitaIn` (RxPdo) | `OrbitaState` (TxPdo) | `OrbitaOut` (TxPdo)
| --- | --- | --- | --- |
| `sm_type` | BUFFERED | MAILBOX |  BUFFERED |
| `address` | 1000 | 1200 | 1300 | 
| `name` | `OrbitaIn` |  `OrbitaState` |  `OrbitaOut` |
| **write frequency** | 1kHz | 10Hz |  1kHz |
| **orbita2d size** | 43 Bytes | 31 Bytes | 27 Bytes |
| **orbita3d size** | 63 Bytes | 45 Bytes | 39 Bytes |

**Some important notes** 
- LAN9252 limits the number of PDO objects supported to 4, so we could potentially add one more PDO object to the list.
- LAN9252 limits the number of bytes that can be written to its memory at once to 64 bytes (otherwise we need to write the data in 64B chunks). This is why the size of the PDOs is important and they are all under 64 bytes.
- Therea are two types of Sync Managers (`sm_type`) used with the firmware for EtherCAT communiction: `BUFFERED` and `MAILBOX`. The `BUFFERED` type is used for the `OrbitaIn` PDOs, because we want to send the data as fast as possible. The `MAILBOX` type is used for the `OrbitaState` and `OrbitaOut` PDOs, because we want to send the data at a slower rate. `BUFFERED` type buffers the data in the master and we do not see any potential data loss if the slave is not able to read/write the data in time. 
`MAILBOX` type uses a handshake mechanism to ensure that the data is received by the master and is mostly used for punctual data that is not time sensitive.

### OrbitaIn (RxPdo)  - Master to Slave

| Entry Name | Entry Type | Index | Sub-Index | 
| --- | --- | --- | --- |
| controlword | UINT16 | 0x6041 | - |
| mode_of_operation | UINT8 | 0x6060 | - |
| target_position | REAL | 0x607A | 1, 2, ... (up to orbita_type) |
| target_velocity | REAL | 0x60FF | 1, 2, ... (up to orbita_type) |
| velocity_limit | REAL | 0x607F | 1, 2, ... (up to orbita_type) |
| target_torque | REAL | 0x6071 | 1, 2, ... (up to orbita_type) |
| torque_limit | REAL | 0x6072 | 1, 2, ... (up to orbita_type) |

### OrbitaState (TxPdo)  - Slave to Master

| Entry Name | Entry Type | Index | Sub-Index |
| --- | --- | --- | --- |
| error_code | UINT16 | 0x603F | 0 (homing), 1,2, .. (up to orbita_type) |
| actuator_type | UINT8 | 0x6402 | - |
| axis_position_zero_offset | REAL | 0x607C | 1, 2, ... (up to orbita_type) |
| board_temperatures | REAL | 0x6500 | 1, 2, ... (up to orbita_type) |
| motor_temperatures | REAL | 0x6501 | 1, 2, ... (up to orbita_type) |

### OrbitaOut (TxPdo) - Slave to Master



| Entry Name | Entry Type | Index | Sub-Index |
| --- | --- | --- | --- |
| statusword | UINT16 | 0x6040 | - |
| mode_of_operation_display | UINT8 | 0x6061 | - |
| actual_position | REAL | 0x6064 | 1, 2, ... (up to orbita_type) |
| actual_velocity | REAL | 0x606C | 1, 2, ... (up to orbita_type) |
| actual_torque | REAL | 0x6077 | 1, 2, ... (up to orbita_type) |
| actual_axis_position | REAL | 0x6063 | 1, 2, ... (up to orbita_type) |
