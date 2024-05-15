# Poulpe ehtercat controller

This is the code that manages the communication between the Poulpe and the ethercat master. The code is written in rust.

There are four main crates in the code:
- `ethercat_controller`: This is the main crate that does the heavy lifting of the communication with the ethercat master. It is a wrapper around the `ethercat-rs` crate. This crate enables to create the ethercat master form an ESI xml file.
- `poulpe_ethercat_controller`: This is an abstraction layer on top of the `ethercat_controller` crate. It provides a more user friendly interface to the user with specific functions for poulpe boards.
- `poulpe_ethercat_multiplexer`: This crate uses the `poulpe_ethercat_controller` to allow for reading assynchronously from multiple poulpe boards connected to the same ethercat master. It is based on the `grpc` protocol. It allows for creating a single server that can be accessed by multiple clients.
- `python_client`: This is a python wrapper of the `poulpe_ethercat_multiplexer` crate's client side. It allows for reading from multiple poulpe boards connected to the same ethercat master from python and in that way enables quick prototyping.