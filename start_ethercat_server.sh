#! /bin/bash

## kill the old server if already running
## this line should not do anything in the normal circumstances
## TODO: make sure that the server never stays on
pkill "server"

## start the grpc server
cd $HOME/dev/poulpe_ethercat_controller
RUST_LOG=info ./target/release/server config/robot.yaml
