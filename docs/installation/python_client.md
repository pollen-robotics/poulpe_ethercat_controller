---
title: Python clinet bindings
layout: default
parent: Installation and configuration
---

# Python GRPC client bindings

The `poulpe_ethercat_controller` crate provides a GRPC server and client interface that can be accessed by multiple clients at the same time, either in Rust or Python. The GRPC server is implemented in Rust and the client bindings are generated for both Rust and Python. The python bindings are implemented in `python_client` crate.

The simplest way of obtaining the python client is to pip install the code directly from the git the repository, using the tag corresponding to the version of the firmware that you are using. For example, if we use the 1.5.4 tag: (see the [tags](https://github.com/pollen-robotics/poulpe_ethercat_controller/tags))

```shell
pip install git+https://github.com/pollen-robotics/poulpe_ethercat_controller.git@1.5.4#subdirectory=python_client
```

NOTE:
This install procedure is available from the version 1.5.4 of the `poulpe_ethercat_controller`.


## Installation from source

Provided that you already have the `poulpe_ethercat_controller` crate cloned and built, you can install the python client bindings from the source code. You can find more information on how to build the code in the <a href="../">installation and configuration</a> section.


First place yourself in the `python_client` directory, and then you can build the python client bindings you can use the `maturin` tool.

```bash
maturin develop --release
```

INFO:
> You will hav to use a virtual environment to install the client with the `maturin` command. So make sure to activate the virtual environment before running the command. We suggest using `conda` to create the virtual environment, but you can use `venv` or `virtualenv` as well. 
> 
```bash
conda create -n poulpe_ethercat python=3.10
conda activate poulpe_ethercat
pip install maturin
```

Once you have the virtual environment activated you can run the following command:
```bash
pip install maturin
maturin develop --release
```

This will install the python client in the current environment.
And then you can open the notebooks in the `notebooks` folder and scripts in the `scripts` folder to see how to use the client. See the [notebooks](https://github.com/pollen-robotics/poulpe_ethercat_controller/tree/develop/python_client/notebooks) for more information.

The client is a wrapper around the GRPC client generated from the `poulpe_ethercat_grpc/src/client.rs` folder.

## Simple example

Create a simple config file `file_config.yaml` with the following content:
```yaml
ethercat:
  master_id: 0
  cycle_time_us: 1000 # us - cycle time of the ethercat 1/frequency
  command_drop_time_us: 5000 # us (5ms default) 
  watchdog_timeout_ms: 500 # ms (500ms default)
  mailbox_wait_time_ms: 10000 #ms  (1s default)
```

And then you can run the following python script:

```python
from python_client import PyPoulpeRemoteClient, launch_server, get_all_slaves_in_network
import time

# launch the server first
address = launch_server("file_config.yaml")

# wait for the server to start
time.sleep(1.0)

# print all slaves in the network
devices = get_all_slaves_in_network(address)
print("Devices connected in the network: ")
for i,name in zip(devices[0],devices[1]):
    print("Slave {}: {}".format(i,name))


# connect to the server
slave_id = 0
no_axis = 3

print('Connecting on slave: {}'.format(slave_id))
# Create an instance of the client
client = PyPoulpeRemoteClient(address, [slave_id], 0.001)

time.sleep(1.0)

devices = client.get_all_slaves_in_network()
print("Devices connected in the network: ")
for i,name in zip(devices[0],devices[1]):
    print("Slave {}: {}".format(i,name))
    
print("Slave {} compliancy is: {}".format(slave_id, client.get_torque_state(slave_id)))
print("Slave {} current position: {}".format(slave_id, client.get_position_actual_value(slave_id)))
```
which might output something like:
```shell
POULPE controller ready!
Devices connected in the network: 
Slave 1: RightWristOrbita3d
Slave 0: RightShoulderOrbita2d
Connecting on slave: 0
Slave 0 compliancy is: True
Slave 0 current position: [-0.0011222249595448375, 3.743586057680659e-05, 6.8065196501265746e-06]
```


NOTE: 
- <i class="fa fa-book fa-lg"></i> You can find the complete list of functions in the `python_client` crate in the [python_client docs](../../api/python_client).
- You can find mure examples of useing the python client in the `python_client/notebooks` directory. [See the notebooks](https://github.com/pollen-robotics/poulpe_ethercat_controller/tree/develop/python_client/notebooks)