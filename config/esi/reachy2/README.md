# Reacy2 ESI files

This directory has all the necessary EtherCAT slave information (ESI) files for the orbita joints of the reachy2 roobt. 
It contrains
- XML ESI files `*.xml`
- compiles ESI file `*.bin`
- complete robot XML ESI file generator `generate_robot_esi_files.py`
- bash compilation script for the XML files `compile_esi.sh`

ESI files are a bit different for different firmware_Poulpe versions. 

firmware version | ESI folder
--- | ---
1.0 | [firmware1.0](firmware1.0)
1.1 | [firmware1.5](firmware1.5)