---
title: Home
layout: default
nav_order: 1
---
# Poulpe EhterCAT stack

This is the full EtherCAT stack that manages the communication with the poulpe boards through EtherCAT network. The code is written in rust. 
It is intended to communicate with poulpe boards running the [firmware_Poulpe](https://github.com/pollen-robotics/firmware_Poulpe).

## Crate structure

The code is based on the [EtherCAT IgH stack](https://gitlab.com/etherlab.org/ethercat). The EtherCAT master is implemented in the `ethercat_controller` crate. The `poulpe_ethercat_controller` crate is the main crate that manages the communication with the poulpe boards. The `poulpe_ethercat_grpc` crate is the crate that manages the communication with the GRPC server. Read more about the crate structure in [Crates docs](software)

## Safety features

The code implements many safety features in order to ensure the safe operation of the boards. Read more about the safety features in the [Safety features](safety_features) docs.


## Install and build the `poulpe_ethercat_controller` code

Now that you have the ethercat master running and the poulpe board configured, you can run the code.

- Clone the repo
```shell
git clone git@github.com:pollen-robotics/poulpe_ethercat_controller.git
```

For more information on how to install and build the code read the [Installation and configuration](installation) docs.

## Support

This project adheres to the Contributor [code of conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to [contact@pollen-robotics.com](mailto:contact@pollen-robotics.com).

Visit [pollen-robotics.com](https://pollen-robotics.com) to learn more or join our [Dicord community](https://discord.gg/vnYD6GAqJR) if you have any questions or want to share your ideas.
Follow [@PollenRobotics](https://twitter.com/pollenrobotics) on Twitter for important announcements.
