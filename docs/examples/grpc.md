---
title: GRPC server and client
parent: Running the code
layout: default
back_to_top: true
back_to_top_text: "Back to top"
---

# GRPC server and client

The `poulp_ethercat_grpc` crate is a GRPC server that can be used to communicate with the poulpe boards connected to the network. The server can be accessed by multiple clients at the same time. The server is written in Rust and the clients can be written in Rust or Python.

<details open markdown="block">
  <summary>
    Table of contents
  </summary>
  {: .text-delta }
1. TOC
{:toc}
</details>

## Running the GRPC server




- Run the GRPC server code

```shell
RUST_LOG=info cargo run --release # add the yaml config file ex. config/robot.yaml
```

- The yaml file contains the configuration of the ethercat network and the poulpe boards connected to the network. The typical yaml file is located in the `config` directory. See the [config/README.md](config/README.md) for more info.
- The GPRC server runs at the ip address of your pc with the port `50098` (ex. `192.168.0.67:50098`)

- Once the server is up and running you can connect to it with the GRPC client, either directly from the examples in this repo or through the ROS stack using `orbita2d_control` or `orbita3d_control` packages.

<details markdown="1"><summary>Example of running the server with the Orbita3d yaml file</summary>

For example, if you have one one poulpe board connected to the network (ex. one orbita3d with the name of `NeckOrbita3d`), and you define your yaml file for example:

```yaml
ethercat:
  master_id: 0
  cycle_time_us: 1000 # us - cycle time of the ethercat 1/frequency
  command_drop_time_us: 5000 # us (5ms default) 
  watchdog_timeout_ms: 500 # ms (500ms default)
  mailbox_wait_time_ms: 10000 #ms  (1s default)
```

You can run the server with:

```shell
$ RUST_LOG=info cargo run --release -- config/my_network_config.yaml # or config/robot.yaml for defualt config

[2024-12-03T07:58:37Z INFO  ethercat_controller::ethercat_controller] Found 1 slaves
[2024-12-03T07:58:37Z INFO  ethercat_controller::ethercat_controller] Slave "NeckOrbita3d" at position 0
[2024-12-03T07:58:37Z INFO  server] Setup Slave 0...
[2024-12-03T07:58:37Z INFO  ethercat_controller::ethercat_controller] Master and all slaves operational!
[2024-12-03T07:58:37Z INFO  poulpe_ethercat_controller] Slave 0, inital state: SwitchOnDisabled
[2024-12-03T07:58:37Z INFO  poulpe_ethercat_controller] Slave 0, setup done! Current state: SwitchedOn
[2024-12-03T07:58:37Z INFO  server] Done!
[2024-12-03T07:58:37Z INFO  server] POULPE controller ready!
[2024-12-03T07:58:47Z INFO  ethercat_controller::ethercat_controller] EtherCAT loop: 913.37 Hz
```

</details>

### Rust GRPC client examples

- The simplest example is the `client_listener` example. This example connects to the server and listens for the states of the poulpe boards connected to the network.

```shell
RUST_LOG=info cargo run --release --example client_listener # add the slave id (ex. 0) or slave name (ex. LeftWristOrbita3d)
```

ex.
```shell
$ RUST_LOG=info cargo run --release --example client_listener 0 # slave id 0

[2024-12-03T08:21:11Z INFO  client_listener] Slave id: 0
[2024-12-03T08:21:13Z INFO  client_listener] Slave ids in network: ([0], ["NeckOrbita3d"])
[2024-12-03T08:21:13Z INFO  client_listener] Compliant: false,	 Target position: [0.0, 0.0, 0.0],	 Current position: [0.441356, 0.43080845, -3.891907]
[2024-12-03T08:21:13Z INFO  client_listener] Compliant: false,	 Target position: [0.0, 0.0, 0.0],	 Current position: [0.441356, 0.43080845, -3.891907]
[2024-12-03T08:21:13Z INFO  client_listener] Compliant: false,	 Target position: [0.0, 0.0, 0.0],	 Current position: [0.441356, 0.43080845, -3.891907]
...

```

- Another example is the `client_sinus` example. This example connects to the server and sends a simple sinusoide trajectory to the poulpe boards connected to the network.

```shell
RUST_LOG=info cargo run --release --example client_sinus # add the slave id (ex. 0) or slave name (ex. LeftWristOrbita3d)
```


### Python GRPC client

- The `poulpe_ethercat_grpc` crate has a python client that can be used to connect to the GRPC server and read the states of the poulpe boards connected to the network. The python client is a wrapper around the GRPC client that is generated in the `python_client` crate. See the [python_client/README.md](python_client/README.md) for more info.
- The python client uses the `maturin` package to build the python wheel.
- See the [python_client/README.md](python_client/README.md) for instructions on how to build the python client.


Once you have your python bindings you can run the examples from the `python_client/scripts` directory or notebooks from the `python_client/notebooks` directory.

Or you can write your own python scripts to interact with the poulpe boards connected to the network. For example

```python
from python_client import PyPoulpeRemoteClient
import time

slave_id = 0
no_axis = 3

print('Connecting on slave: {}'.format(slave_id))
# Create an instance of the client
client = PyPoulpeRemoteClient("http://127.0.0.1:50098", [slave_id], 0.001)

time.sleep(1.0)

print("Connected slaves to master: {}".format(client.get_connected_devices()))

print("Slave {} compliancy is: {}".format(slave_id, client.get_torque_state(slave_id)))
print("Slave {} current position: {}".format(slave_id, client.get_position_actual_value(slave_id)))
```
which might output something like:
```shell
Connecting on slave: 0
Connected slaves to master: ([0], ['NeckOrbita3d'])
Slave 0 compliancy is: True
Slave 0 current position: [-0.0011222249595448375, 3.743586057680659e-05, 6.8065196501265746e-06]
```

### Orbita2d and Orbita3d control clients

- Once you have your GRPC server running you can connect to it using the
    - `orbita2d_control` - [see on github](https://github.com/pollen-robotics/orbita2d_control)
    - `orbita3d_control` - [see on github](https://github.com/pollen-robotics/orbita3d_control)
- The ROS packages are used to control the orbita2d and orbita3d actuators, implementing the kinematics of the actuators enabling to control them in joint space or cartesian space.
- Also, `orbita2d_control` and `orbita3d_control` are ROS packages that use the `poulpe_ethercat_grpc` crate to connect to the GRPC server and control the poulpe boards connected to the network.