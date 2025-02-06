---
title: config
layout: default
parent: Crates
nav_order: 5
---

# Configuration 

This is a directory that contains the configuration files for EtherCAT network and the poulpe boards. 

<details open markdown="block">
  <summary>
    Table of contents
  </summary>
  {: .text-delta }
1. TOC
{:toc}
</details>

## EtherCAT network and GRPC server configuration

It contains an example yaml file configuration for the EtherCAT network `ethercat.yaml` which determins few important varaibles for GRPC server and the EtherCAT master. See more info in the [Running the code](../../examples/grpc#running-the-grpc-server) docs.
```yaml
ethercat:
  master_id: 0
  cycle_time_us: 1000 # us
  command_drop_time_us: 5000 # us (5ms default)
  watchdog_timeout_ms: 500 # ms (500ms default)
  mailbox_wait_time_ms: 10000 #ms  (1s default)
```

The contens of the `yaml` file are:
- `master_id`: The id of the EtherCAT master - usually 0 
- `cycle_time_us`: The cycle time of the EtherCAT master in microseconds. The PDOs will be read and written in this time interval ( frequncy = 1/cycle_time_us).
- `command_drop_time_us`: The time in microseconds at which the GRPC server will consider that teh GRPC client's command is too old and drop it. 
- `watchdog_timeout_ms`: The time in milliseconds that the EtherCAT master waits for the response for the slave to update the wathcdog (it should do it at the frequency of the cycle time). If it does not update the watchdog in time, the master will consider the slave not operational and will stop the operation.
- `mailbox_wait_time_ms`: The time in milliseconds that the EtherCAT master waits for the response for the slave to update the mailbox PDOs. If the slave does not update the mailbox PDOs in time, the master will consider the slave not operational and will stop the operation. It is only used if the `verify_mailbox_pdos` feature is enabled in the `ethercat_controller` crate and the mailbox PDOs are used.

## Poulpe boards configuration

In order to use poulpe boards with the EtherCAT network, the boards need to be configured properly. More precisely the LAN9252 chip on the board needs to be configured properly. The configuration is done using the ESI XML file that are compiled to their binary version and flashed to the EEPROM of the LAN9252 chip. The configuration files are located in the `config/esi` directory.

See the guide how to configure the poulpe boards on the network in the [Configure Poulpes](../../installation/configure_poulpe) docs.