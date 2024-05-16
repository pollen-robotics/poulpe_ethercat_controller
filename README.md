# Poulpe ehtercat controller

This is the code that manages the communication between the Poulpe and the ethercat master. The code is written in rust.

There are four main crates in the code:
- `ethercat_controller`: This is the main crate that does the heavy lifting of the communication with the ethercat master. It is a wrapper around the `ethercat-rs` crate. This crate enables to create the ethercat master form an ESI xml file.
- `poulpe_ethercat_controller`: This is an abstraction layer on top of the `ethercat_controller` crate. It provides a more user friendly interface to the user with specific functions for poulpe boards.
- `poulpe_ethercat_multiplexer`: This crate uses the `poulpe_ethercat_controller` to allow for reading assynchronously from multiple poulpe boards connected to the same ethercat master. It is based on the `grpc` protocol. It allows for creating a single server that can be accessed by multiple clients.
- `python_client`: This is a python wrapper of the `poulpe_ethercat_multiplexer` crate's client side. It allows for reading from multiple poulpe boards connected to the same ethercat master from python and in that way enables quick prototyping.


## Installing Ethercat on the PC

See the notion: https://www.notion.so/pollen-robotics/Setup-EtherCAT-1ecce786847e495bb1b4b399740727af

Installing the ehtercat master

- Install the dependencies (on ubuntu):
    - sudo apt install autoconf libtool
- Install the [ethercat master](https://etherlab.org/en/ethercat/)
    - `git clone https://gitlab.com/etherlab.org/ethercat.git`
    - `./bootstrap`
    - `./configure --enable-generic --disable-8139too`
    - `make all modules`
    - `sudo make modules_install install`
    - `sudo depmod`

See the  for more info.

- See in the [ethercat docs](https://etherlab.org/download/ethercat/ethercat-1.5.2.pdf) hov to configure the dev rules for `/dev/EtherCAT0`(go to the mode 0666)
- Modify the file `/usr/local/etc/ethercat.conf`
    - `MASTER0_DEVICE` - set the mac address of the eth0 (can be found using `ip a`)
    - `DEVICE_MODULES` set to `”generic”`

### Quickstart ethercat master

- `sudo ethercatctl start`
- verify that `/dev/EtherCAT0` exists
- Use `ethercat` (installed with `make install` after the compilation) to veviry is the master is working
    - ex. `ethercat graph` (list of nodes connected)
    - ex. `ethercat slaves` (list of slaves connected)