---
title: Standalone examples
layout: default
back_to_top: true
back_to_top_text: "Back to top"
parent: Running the code
---



# Standalone examples

This page shows how to use the `poulpe_ethercat_controller` crate to communicate with the poulpe boards connected to the network. The examples are standalone and do not require the GRPC server to be running.


<details open markdown="block">
  <summary>
    Table of contents
  </summary>
  {: .text-delta }
1. TOC
{:toc}
</details>


### Scan the network

- Scan the network to find the poulpe boards connected to the network

```shell
RUST_LOG=info cargo run --release --example scan_network
```

ex.

```shell
$ RUST_LOG=info cargo run --release --example scan_network

[2024-12-03T07:36:38Z INFO  network_scan] Creating the therCAT master
[2024-12-03T07:36:38Z INFO  ethercat_controller::ethercat_controller] Found 1 slaves
[2024-12-03T07:36:38Z INFO  ethercat_controller::ethercat_controller] Slave "NeckOrbita3d" at position 0
[2024-12-03T07:36:38Z INFO  network_scan] Waiting for therCAT master to be ready
[2024-12-03T07:36:38Z INFO  network_scan] EtherCAT master is ready
[2024-12-03T07:36:38Z INFO  network_scan] ---------------------------
[2024-12-03T07:36:38Z INFO  network_scan] Scanning network
[2024-12-03T07:36:38Z INFO  network_scan] Slave ID: 0, name: NeckOrbita3d

```

### Read poulpe states

- Read the state of the poulpe boards connected to the network

```shell
RUST_LOG=info cargo run --release --example poulpe_read_states # add the slave id ex. 0
```

ex.

```shell
$ RUST_LOG=info cargo run --release --example poulpe_read_states 0

[2024-12-03T07:52:18Z INFO  poulpe_read_states] Loading the controller
[2024-12-03T07:52:18Z INFO  ethercat_controller::ethercat_controller] Found 1 slaves
[2024-12-03T07:52:18Z INFO  ethercat_controller::ethserverercat_controller] Slave "NeckOrbita3d" at position 0
[2024-12-03T07:52:18Z INFO  poulpe_read_states] Waiting for controller to be ready
[2024-12-03T07:52:18Z INFO  poulpe_read_states] Controller is ready
[2024-12-03T07:52:18Z INFO  ethercat_controller::ethercat_controller] Master and all slaves operational!
[2024-12-03T07:52:20Z INFO  poulpe_read_states] Pos: [0.44092038, 0.43091735, -3.8916345], 	 Vel: [9.1e-44, -8e-45, -7.533014e-5],	 Axis: [0.42992437, 0.41074842, 2.40121], 	 Board Temp: [38.634876, 38.82775, 38.124046], 	 Motor Temp: [-273.15, -273.15, -273.15]
[2024-12-03T07:52:20Z INFO  poulpe_read_states] Pos: [0.44092038, 0.43091735, -3.8916345], 	 Vel: [9.1e-44, -8e-45, -7.533014e-5],	 Axis: [0.42992437, 0.41074842, 2.40121], 	 Board Temp: [38.634876, 38.82775, 38.124046], 	 Motor Temp: [-273.15, -273.15, -273.15]
[2024-12-03T07:52:20Z INFO  poulpe_read_states] Pos: [0.44092038, 0.43091735, -3.8916345], 	 Vel: [9.1e-44, -8e-45, -7.533014e-5],	 Axis: [0.42992437, 0.41074842, 2.40121], 	 Board Temp: [38.634876, 38.82775, 38.124046], 	 Motor Temp: [-273.15, -273.15, -273.15]
[2024-12-03T07:52:20Z INFO  poulpe_read_states] Pos: [0.44092038, 0.43091735, -3.8916345], 	 Vel: [9.1e-44, -8e-45, -7.533014e-5],	 Axis: [0.42992437, 0.41074842, 2.40121], 	 Board Temp: [38.634876, 38.82775, 38.124046], 	 Motor Temp: [-273.15, -273.15, -273.15]
[2024-12-03T07:52:20Z INFO  poulpe_read_states] Pos: [0.44092038, 0.43091735, -3.8916345], 	 Vel: [9.1e-44, -8e-45, -7.533014e-5],	 Axis: [0.42992437, 0.41074842, 2.40121], 	 Board Temp: [38.634876, 38.82775, 38.124046], 	 Motor Temp: [-273.15, -273.15, -273.15]

```

### Running a simple sinusoide trajectory

- Run a simple sinusoide trajectory on the poulpe boards connected to the network

```shell
RUST_LOG=info cargo run --release --example poulpe_sinus # add the slave id ex. 0
```

ex.

```shell
$ RUST_LOG=info cargo run --release --example poulpe_sinus 0

[2024-12-03T07:52:18Z INFO  poulpe_read_states] Loading the controller
[2024-12-03T07:52:18Z INFO  ethercat_controller::ethercat_controller] Found 1 slaves
[2024-12-03T07:52:18Z INFO  ethercat_controller::ethercat_controller] Slave "NeckOrbita3d" at position 0
[2024-12-03T07:52:18Z INFO  poulpe_read_states] Waiting for controller to be ready
[2024-12-03T07:52:18Z INFO  poulpe_read_states] Controller is ready
[2024-12-03T07:52:18Z INFO  ethercat_controller::ethercat_controller] Master and all slaves operational!
...
```

