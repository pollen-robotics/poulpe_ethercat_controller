---
title: python_client
layout: default
nav_order: 6
parent: Crates
---

## Python GRPC client for comunicating with Ethercat Master 

To use this client run the following command:

```bash
maturin develop --release
```

> IMPORTANT: We suggest using a conda environment to install the client. 

```bash
conda create -n poulpe_ethercat python=3.8
conda activate poulpe_ethercat
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
