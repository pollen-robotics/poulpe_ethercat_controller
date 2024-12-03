from python_client import PyPoulpeRemoteClient
import numpy as np
from glob import glob
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

print("Connected slaves: {}".format(client.get_connected_devices()))

client.turn_off(slave_id);

np.set_printoptions(precision=15)
time.sleep(0.01)
if no_axis == 3:
    print(f"orbita3d zeros: {np.array(client.get_axis_sensors(slave_id), dtype=float)*12.0/64.0}")
else:
    print(f"orbita2d zeros: {client.get_axis_sensors(slave_id)}")