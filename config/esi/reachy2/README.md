# Reacy2 ESI files

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

