## Python GRPC client for comunicating with Ethercat Master 

To use this client run the following commands:

```bash
maturin develop
```

This will install the python client in the current environment.
And then you can open the notebooks in the `notebooks` folder to see how to use the client.

The client is a wrapper around the GRPC client generated from the `poulpe_ethercat_grpc/client` folder.

Make sure to run the GRPC server before running the client.
This can be done using the following command:

```bash
cargo run --release ../config/file/here.yaml
```