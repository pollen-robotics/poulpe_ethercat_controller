from python_client import PyPoulpeRemoteClient
import numpy as np
import sys
import time

#get the id and the no_axis from the arguments
if len(sys.argv) != 3:
    print("Error: Invalid number of arguments")
    print("Usage: python read_orbita_zeros.py <id> <no_axis>")
    sys.exit()
else:
    slave_id = int(sys.argv[1])
    no_axis = int(sys.argv[2])

print('Connecting on slave: {}'.format(slave_id))
# Create an instance of the client
client = PyPoulpeRemoteClient("http://127.0.0.1:50098", [slave_id], 0.001)

time.sleep(1.0)

devices = client.get_all_slaves_in_network()
print("Devices connected in the network: ")
for i,name in zip(devices[0],devices[1]):
    print("Slave {}: {}".format(i,name))
    
print("Slave {} compliancy is: {}".format(slave_id, client.get_torque_state(slave_id)))
print("Slave {} current position: {}".format(slave_id, client.get_position_actual_value(slave_id)))