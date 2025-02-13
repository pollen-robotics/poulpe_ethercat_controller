import python_client
import time 

print("Launching the EhterCAT server")

# launch the server
address = python_client.launch_server("../../config/ethercat.yaml")
time.sleep(1)

# print all slaves in the network
devices = python_client.get_all_slaves_in_network(address)
print("Devices connected in the network: ")
for i,name in zip(devices[0],devices[1]):
    print("Slave {}: {}".format(i,name))
    
    
while True:
    pass