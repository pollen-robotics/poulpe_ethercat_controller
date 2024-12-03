## EtherCAT configuration files

This directory contains the Ethercat Slave Information (ESI) files used to configure the Ethercat slaves in the `esi` folder and the `yaml` files used to configure the Ethercat master.
- See more about the ESI files in the [ESI README](esi/README.md)


### Ethercat master configuration yaml files

The `yaml` files in this directory are used to 
- configure the Ethercat master
- fully define the Ethercat network topology
    - slave ids
    - slave names
    - slave types

The `yaml` files have the following structure:

```yaml
ethercat:
  master_id: 0
  cycle_time_us: 1000 # us
  command_drop_time_us: 5000 # us (5ms default)
  watchdog_timeout_ms: 500 # ms (500ms default)
  mailbox_wait_time_ms: 10000 #ms  (1s default)

slaves:
  - !Poulpe
    id: 0
    orbita_type: 3
    name: "NeckOrbita3d"
  - !Poulpe
    id: 1
    orbita_type: 2
    name: "LeftShoulderOrbita2d"
  - !Poulpe
    id: 2
    orbita_type: 3
    name: "LeftWristOrbita3d"
  - !Poulpe
    id: 3
    orbita_type: 2
    name: "LeftElbowOrbita2d"
  - !Poulpe
    id: 4
    orbita_type: 2
    name: "RightShoulderOrbita2d"
  - !Poulpe
    id: 5
    orbita_type: 3
    name: "RightWristOrbita3d"
  - !Poulpe
    id: 6
    orbita_type: 2
    name: "RightElbowOrbita2d"
```

The `ethercat` section defines the master configuration:
- `master_id` : the master id
- `cycle_time_us` : the ethercat cycle time in microseconds (the frequency of the ethercat network)
- `command_drop_time_us` : the time in microseconds after which the master will drop a command received from the GRPC client as it is too old
- `watchdog_timeout_ms` : the time in milliseconds after which the master will consider a slave as disconnected and will stop the network
- `mailbox_wait_time_ms` : the time in milliseconds the master will wait for a response from the slave before considering the slave as disconnected and stopping the network

The `slaves` section defines the slaves in the network:
- `id` : the slave id
- `orbita_type` : the type of orbita slave (2 for orbita2d, 3 for orbita3d)
- `name` : the name of the slave

