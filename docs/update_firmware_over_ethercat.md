---
title: Update Firmware over EtherCAT
layout: default
---

# Update Firmware over EtherCAT

You'll need to have the poulpe board with the `firmware_Poulpe` version of at least `v1.5.x` to be able to update the firmware over EtherCAT.


## Update the firmware

Once when you have your `firmware_Poulpe` compiled and trasnformed to the binary file, you can update the firmware on the poulpe board over the EtherCAT network.


### Using the EtherCAT client

You can transfer the firmware binary file to the poulpe board using the `ethercat` client:

```shell
ethercat foe_write -p0 firmware.bin --verbose # it will write the firmware to the slave with id 0
```

Once when this is done, you can send the SDO request the the poulpe board to complete the firmware update. 
The firmware will wait for on the SDO index `0x100` and the subindex `1` for the value that corresponds to the file size of the firmware in bytes. 
You can get the fie size in bytes with the following command:
```shell
stat -c %s firmware.bin
```

Then you can send the SDO request with the following command (ex. 1000 bytes):

```shell
ethercat sdo_write -p0 0x100 1 -t uint32 1000 # it will write the value 1000 to the SDO index 0x100 and subindex 1 of the slave with id 0
```

<details markdown="1"><summary><b>Read the number of bytes received by the poulpe board</b></summary>

You can also test how many bytes are already written to the poulpe board by reading the SDO index `0x100` and subindex `1`:

```shell
ethercat sdo_read -p0 0x100 1 -t uint32 # it will read the value of the SDO index 0x100 and subindex 1 of the slave with id 0
```

You should have exactly the same number of bites as the file size that you have written to the board.

</details>


The firmware will then start the update process. Once when the update is done, the poulpe board will restart and the new firmware will be loaded.

### Using the update_firmware script

You can also use the `update_firmware` script that is located in the `poulpe_ethercat_controller` crate.

```shell
sh update_firmware.sh 0 firmware.bin # it will update the firmware of the slave with id 0 with the firmware.bin file
```

The script will automatically send the firmware to the baord, verify that it has been well received and then send the SDO request to start the update process.
