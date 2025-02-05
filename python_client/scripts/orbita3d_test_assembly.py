from python_client import PyPoulpeRemoteClient
import time
import numpy as np
import sys

#get the id and the no_axis from the arguments
if len(sys.argv) != 3:
    print("Error: Invalid number of arguments")
    print("Usage: python orbita3d_test_assembly.py <id> <no_axis>")
    sys.exit()
else:
    slave_id = int(sys.argv[1])
    n_axis = int(sys.argv[2])

# Create an instance of the client
client = PyPoulpeRemoteClient("http://127.0.0.1:50098", [slave_id], 0.001)

print(client.get_connected_devices())

# Use client methods
client.turn_on(slave_id)

client.set_target_position(slave_id,[0]*n_axis)
client.set_torque_limit(slave_id,[0.3]*n_axis)
client.set_velocity_limit(slave_id, [0.3]*n_axis)
 
time.sleep(5.0)

print("Startting the test")

client.set_torque_limit(slave_id,[1.0]*n_axis)
client.set_velocity_limit(slave_id, [1.0]*n_axis)

print("Rotate 360 degrees")

t0 = time.time()
stop = False
tar, t=[],[]
pos, vel, torque, axis_sensors = [], [], [], []
target = 0
while target < 5.3333333*2*np.pi:
    target = target+0.004
    client.set_target_position(slave_id,[target]*n_axis)
    time.sleep(0.001)
    t.append(time.time()-t0)
    tar.append(client.get_target_position(slave_id))
    pos.append(client.get_position_actual_value(slave_id))
    vel.append(client.get_velocity_actual_value(slave_id))
    torque.append(client.get_torque_actual_value(slave_id))
    axis_sensors.append(client.get_axis_sensors(slave_id))

print("Rotate back to 0 degrees")
while target > 0:
    target = target-0.004
    client.set_target_position(slave_id,[target]*n_axis)
    time.sleep(0.001)
    t.append(time.time()-t0)
    tar.append(client.get_target_position(slave_id))
    pos.append(client.get_position_actual_value(slave_id))
    vel.append(client.get_velocity_actual_value(slave_id))
    torque.append(client.get_torque_actual_value(slave_id))
    axis_sensors.append(client.get_axis_sensors(slave_id))
        
tar = np.array(tar)
vel = np.array(vel)
pos = np.array(pos)
torque = np.array(torque)
axis_sensors = np.array(axis_sensors)


import pickle

# filename 
filename = "data/orbita3d_test_assembly_" + str(time.time()) + ".pkl"
with open(filename, 'wb') as f:  # Python 3: open(..., 'wb')
    pickle.dump([t, pos, tar, vel, torque, axis_sensors, n_axis], f)
    print(f"Data saved in {filename}")

print("Test finished, homing")
client.set_torque_limit(slave_id,[0.3]*n_axis)
client.set_velocity_limit(slave_id, [0.3]*n_axis)
client.set_target_position(slave_id,[0]*n_axis)


print("Plotting")
import matplotlib.pyplot as plt

fig, axs = plt.subplots(4,n_axis, figsize=(10,10), sharex=True)

for i, a in enumerate(axs.T):
    a[0].step(t,pos[:,i], label="actual")
    a[0].step(t,tar[:,i], label="target")    
    a[1].step(t,vel[:,i], label = "actual")
    a[2].step(t, np.abs(torque[:,i]), label = "actual")
    a[3].step(t, axis_sensors[:,i], label = "actual")

for i, a in enumerate(axs[:].T):
    a[0].set_ylabel("position")
    a[0].legend()
    a[1].set_ylabel("velocity")
    a[1].legend()
    a[2].set_ylabel("current")
    a[2].legend()
    a[3].set_ylabel("axis position")
    a[3].legend()
    break

plt.legend()
plt.show()

