# Poulpe ehtercat controller

This is the code that manages the communication between the Poulpe and the ethercat master. The code is written in rust.

It is intended to communicate with [poulpe boards](https://github.com/pollen-robotics/elec_Poulpe) running the [firmware_Poulpe](https://github.com/pollen-robotics/firmware_Poulpe).

There are four main crates in the code:
- `ethercat_controller`: This is the main crate that does the heavy lifting of the communication with the ethercat master. It is a wrapper around the `ethercat-rs` crate. This crate enables to create the ethercat master form an ESI xml file.
- `poulpe_ethercat_controller`: This is an abstraction layer on top of the `ethercat_controller` crate. It provides a more user friendly interface to the user with specific functions for poulpe boards.
- `poulpe_ethercat_grpc`: This crate uses the `poulpe_ethercat_controller` to allow for reading assynchronously from multiple poulpe boards connected to the same ethercat master. It is based on the `grpc` protocol. It allows for creating a single server that can be accessed by multiple clients.
- `python_client`: This is a python wrapper of the `poulpe_ethercat_grpc` crate's client side. It allows for reading from multiple poulpe boards connected to the same ethercat master from python and in that way enables quick prototyping.

## Some sytem dependancies for `ethercat-rs`

Ethercat-rs uses the `libclang` and `protobuf` libraries. To install them on ubuntu run:
```
sudo apt-get install libclang-dev
sudo apt-get install -y protobuf-compiler libprotobuf-dev
```

## Installing Ethercat on the PC

See the notion: https://www.notion.so/pollen-robotics/Setup-EtherCAT-1ecce786847e495bb1b4b399740727af

Installing the ehtercat master

- Install the dependencies (on ubuntu):
    - `sudo apt install autoconf libtool` 
- Install the [ethercat master](https://etherlab.org/en/ethercat/)
    - `git clone https://gitlab.com/etherlab.org/ethercat.git`
    - `cd ethercat`
    - use the `stable-1.5` branch `git checkout stable-1.5`
    - `./bootstrap`
    - `./configure --enable-generic --disable-8139too`
    - `make all modules`
    - `sudo make modules_install install`
    - `sudo depmod`
    - add the path to the `ethercat` binary to the `ETHERCAT_PATH` variable (ex. `export ETHERCAT_PATH=$HOME/ethercat`)


See the  for more info.

- See in the [ethercat docs](https://etherlab.org/download/ethercat/ethercat-1.5.2.pdf) how to configure the dev rules for `/dev/EtherCAT0`(go to the mode `0666`). In short it is:
    - create the ethercat rule: `sudo nano /etc/udev/rules.d/99-EtherCAT.rules`
    - add the following line: `KERNEL == "EtherCAT[0-9]*" , MODE ="0666", GROUP ="users"`
    - optionally reload the rules: `sudo udevadm control --reload-rules && sudo udevadm trigger`
- Modify the file `/usr/local/etc/ethercat.conf`
    - `MASTER0_DEVICE` - set the mac address of the eth0 (can be found using `ip a`)
    - `DEVICE_MODULES` set to `”generic”`

### Quickstart ethercat master

- `sudo ethercatctl start`
- verify that `/dev/EtherCAT0` exists
- Use `ethercat` (installed with `make install` after the compilation) to veviry is the master is working
    - ex. `ethercat graph` (list of nodes connected)
    - ex. `ethercat slaves` (list of slaves connected)

## Enabling the ethercat on poulpe

Poulpe boards have LN9252 chip that allows for the communication with the ethercat master. The chip is connected to the STM32H7 microcontroller using SPI. The poulpe board's firmware is configured to communicate with the LN9252 chip and send and receive commands from the ethercat master.
See more info in the [firmware_Poulpe](https://github.com/pollen-robotics/firmware_Poulpe). 

In order to be able to communicate with the maser, the slave have to have proporly configured eeprom of the LN9252 chip. See the procedure in the `config` directory [here](config/README.md)

