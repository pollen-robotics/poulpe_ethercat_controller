import python_client

print("Launching the EhterCAT server")

# launch the server and allow ctrl-c to stop the server
python_client.launch_server("../../config/ethercat.yaml")

while True:
    pass