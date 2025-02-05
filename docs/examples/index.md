---
title: Running the code
layout: default
back_to_top: true
back_to_top_text: "Back to top"
has_children: true
has_toc: false
---

# Running the code

The code in the `poulpe_ethercat_controller` crate can be run in multiple ways. The main ways are:

- [Standalone examples](examples/standalone.md): The examples are standalone and do not require the GRPC server to be running.
- [GRPC server and client](examples/grpc.md): The `poulp_ethercat_grpc` crate is a GRPC server that can be used to communicate with the poulpe boards connected to the network. The server can be accessed by multiple clients at the same time. The server is written in Rust and the clients can be written in Rust or Python.
