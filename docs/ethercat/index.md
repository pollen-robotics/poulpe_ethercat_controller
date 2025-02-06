--- 
title: EtherCAT protocols
layout: default
has_children: true
nav_order: 4
---


# EtherCAT communication

There are several types of communicaiton supported by the EtherCAT protocol. The communication is divided into two main categories:
- Real-time communication
- Non real-time communication 

##  Real-time communication

The real-time variables are exchanged over the network at a high frequency (typically 1kHz) using the PDO communication. There are two types of PDO communication supported by the crate

- Cyclic PDOs - buffered
- Mailbox PDOs - (optional)

{: .info }
> - **PDO** - Process Data Object

Cyclic PDOs are the most common way of exchanging the data over the network. The data is exchanged at a fixed frequency and the data is buffered, ensuring the continuitiy. If the data is not read in time, the data is overwritten with the new data. 

Mailbox PDOs are used to ensure that the data is read and written properly. Mailbox enables a handshake between the master and the slave, ensuring that the data is read and written properly. If the data is not written in time, the master will not read the old data but will read zeros. 

{: .note}
In order to ensure the continuity, mailbox PDOs are buffered in software in the `ethercat_controller` crate, with a timeout of 1s. If the data is not written in time, the master will consider the slave not operational and will fail. This procedure is enabled by default and can be disabled using the feature `verify_mailbox_pdos` in its [Cargo.toml]({{site.github_url}}ethercat_controller/Cargo.toml) file.

##  Non real-time communication - from firmware 1.5.x

The non real-time variables are exchanged over the network using the Mailbox protocol, based on CoE. There are two types of communication supported by the crate.

- SDO communication (Mailbox protocol with CoE)
- FoE communication for file upload - (Mailbox protocol with CoE)

{: .info }
> - **SDO** - Service Data Object
> - **CoE** - Can Over Ethercat
> - **FoE** - Fiile over Ethercat


The SDO communication is used to read and write the data to the slave devices with a handshake. It is mostly used for configuring the slave devices and before starting the real-time communication. In poulpe firmware, it is supported by the `firmware_Poulpe` version 1.5.x and the boards only respond to the SDO communication if they are in the `PREOP` (pre-operational) state.

The FoE communication is used to upload the files to the slave devices, in particular to upload the firmware to the slave devices. In poulpe firmware, it is supported by the `firmware_Poulpe` version 1.5.x and only in `PREOP` (pre-operational) state.

## Communication configuration

The crate determins which kind of communication is available from the ESI XML file downloaded from the slave, and sets up the necessary infrastructure for the communication. See the [poulpe configuration docs](../installation/configure_poulpe) for more info on how to configure the poulpe boards for the EtherCAT network.

There are two versions of the natively supported firmware for the poulpe boards, that have slightly different communication structures:
- `firmware_Poulpe` version 1.0.x - [firmware 1.0](firmware_1_0)
- `firmware_Poulpe` version 1.5.x - [firmware 1.5](firmware_1_5)

{: .note}
Firmware 1.5.x version has the same PDO structure but it has been resturctured to support FoE and CoE communication.