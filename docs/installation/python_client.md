---
title: Python clinet bindings
layout: default
parent: Installation and configuration
---

# Python GRPC client bindings

The `poulpe_ethercat_controller` crate provides a GRPC server and client interface that can be accessed by multiple clients at the same time, either in Rust or Python. The GRPC server is implemented in Rust and the client bindings are generated for both Rust and Python. The python bindings are implemented in `python_client` crate.

## Installation


To use this client run the following command:

```bash
maturin develop --release
```

{: .info }
> You will hav to use a virtual environment to install the client with the `maturin` command. So make sure to activate the virtual environment before running the command. We suggest using `conda` to create the virtual environment, but you can use `venv` or `virtualenv` as well. 
> 
```bash
conda create -n poulpe_ethercat python=3.10
conda activate poulpe_ethercat
```

Once you have the virtual environment activated you can run the following command:
```bash
pip install maturin
maturin develop --release
```

This will install the python client in the current environment.
And then you can open the notebooks in the `notebooks` folder and scripts in the `scripts` folder to see how to use the client.

The client is a wrapper around the GRPC client generated from the `poulpe_ethercat_grpc/src/client.rs` folder.

## Run the GRPC server
Make sure to run the GRPC server before running the client.
This can be done using the following command:

```bash
cargo run --release ../config/file/here.yaml
```

## Simple example

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
