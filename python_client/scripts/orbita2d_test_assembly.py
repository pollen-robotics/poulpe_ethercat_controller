from python_client import PyPoulpeRemoteClient

slave_id = 0
n_axis = 2 # 2 or 3

# Create an instance of the client
client = PyPoulpeRemoteClient("http://127.0.0.1:50098", [slave_id], 0.001)

print(client.get_connected_devices())

client.turn_off(slave_id)
time.sleep(0.5)
client.set_mode_of_operation(slave_id, 1)
time.sleep(0.5)

# Use client methods
client.turn_on(slave_id)
client.set_torque_limit(slave_id,[0.3]*n_axis)
client.set_velocity_limit(slave_id, [0.3]*n_axis)


import numpy as np
import time


client.set_torque_limit(slave_id,[1.0]*n_axis)
client.set_velocity_limit(slave_id, [1.0]*n_axis)

t0 = time.time()
stop = False
tar, t=[],[]
pos, vel, torque, axis_sensors = [], [], [], []
target = 0
while target < 1.9*2*np.pi:
    target = target+0.002
    client.set_target_position(slave_id,[target]*n_axis)
    time.sleep(0.001)
    t.append(time.time()-t0)
    tar.append(client.get_target_position(slave_id))
    pos.append(client.get_position_actual_value(slave_id))
    vel.append(client.get_velocity_actual_value(slave_id))
    torque.append(client.get_torque_actual_value(slave_id))
    axis_sensors.append(client.get_axis_sensors(slave_id))


while target > 0:
    target = target-0.002
    client.set_target_position(slave_id,[target]*n_axis)
    time.sleep(0.001)
    t.append(time.time()-t0)
    tar.append(client.get_target_position(slave_id))
    pos.append(client.get_position_actual_value(slave_id))
    vel.append(client.get_velocity_actual_value(slave_id))
    torque.append(client.get_torque_actual_value(slave_id))
    axis_sensors.append(client.get_axis_sensors(slave_id))
    
while target < 1.9*2*np.pi:
    target = target+0.002
    client.set_target_position(slave_id,[target, -target])
    time.sleep(0.001)
    t.append(time.time()-t0)
    tar.append(client.get_target_position(slave_id))
    pos.append(client.get_position_actual_value(slave_id))
    vel.append(client.get_velocity_actual_value(slave_id))
    torque.append(client.get_torque_actual_value(slave_id))
    axis_sensors.append(client.get_axis_sensors(slave_id))


while target > 0:
    target = target-0.002
    client.set_target_position(slave_id,[target, -target])
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
client.set_torque_limit(slave_id,[0.3]*n_axis)
client.set_velocity_limit(slave_id, [0.3]*n_axis)
client.set_target_position(slave_id,[0]*n_axis)


import pickle
# filename 
filename = "data/orbita2d_test_assembly_" + str(time.time()) + ".pkl"
with open(filename, 'wb') as f:  # Python 3: open(..., 'wb')
    pickle.dump([t, pos, tar, vel, torque, axis_sensors, n_axis], f)
    print(f"Data saved in {filename}")
