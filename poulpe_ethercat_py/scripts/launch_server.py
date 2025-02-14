from poulpe_ethercat_py import PyEthercatServer
import time 

print("Launching the EhterCAT server")

# launch the server
server = PyEthercatServer()
server.launch_server("../../config/ethercat.yaml")
time.sleep(1)

# print all slaves in the network
devices = server.get_all_slaves_in_network()
print("Devices connected in the network: ")
for i,name in zip(devices[0],devices[1]):
    print("Slave {}: {}".format(i,name))
    
    
while True:
    pass