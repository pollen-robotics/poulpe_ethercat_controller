from python_client import PyPoulpeRemoteClient
import time
import numpy as np
import sys

import pickle

# get the file name from the arguments
if len(sys.argv) != 2:
    print("Error: Invalid number of arguments")
    print("Usage: python plot_test_data.py <file_name>")
    sys.exit()

# Getting back the objects:
filename = sys.argv[1]
with open(filename, 'rb') as f:  # Python 3: open(..., 'wb')
    t, pos, tar, vel, torque, axis_sensors, n_axis = pickle.load(f)


print("Plotting")
import matplotlib.pyplot as plt

fig, axs = plt.subplots(3,n_axis, figsize=(10,10), sharex=True)

for i, a in enumerate(axs.T):
    a[0].step(t,pos[:,i], label="actual")
    a[0].step(t,tar[:,i], label="target")    
    a[1].step(t,vel[:,i], label = "actual")
    a[2].step(t, np.abs(torque[:,i]), label = "actual")
    #a[3].step(t, axis_sensors[:,i], label = "actual")

for i, a in enumerate(axs[:].T):
    a[0].set_ylabel("position")
    a[0].legend()
    a[1].set_ylabel("velocity")
    a[1].legend()
    a[2].set_ylabel("current")
    a[2].legend()
    #a[3].set_ylabel("axis position")
    #a[3].set_legend()
    a[0].grid()
    a[2].grid()
    a[1].grid()

plt.show()

def wrap(angle):
    return (angle + 2 * np.pi) % (2 * np.pi)

orbita_3d_ik_mat = np.eye(n_axis)
axis_readings_initial = np.array(axis_sensors[0,:]).reshape(-1,1)

print(axis_readings_initial)
axis_calc = (pos@orbita_3d_ik_mat).T + np.array(axis_readings_initial) % (2*np.pi)
axis_calc = (wrap(axis_calc))

axis_error = axis_calc-axis_sensors.T
for i, ax_e in enumerate(axis_error):
    for j,a in enumerate(ax_e):
        if np.abs(a) > np.pi:
            axis_error[i,j] = a - (np.sign(a))*2*np.pi


fig, axs = plt.subplots(2,n_axis, figsize=(10,10))

for i, a in enumerate(axs.T):
    a[0].step(t, np.rad2deg(axis_sensors[:,i]), label = "actual [deg]")
    a[0].step(t, np.rad2deg(axis_calc[i,:]), label = "estimated [deg]")
    a[1].step(t, np.rad2deg(axis_error[i,:]), label = "backlash [deg]")

for i, a in enumerate(axs[:].T):
    a[0].set_ylabel("axis position")
    a[0].legend()
    a[0].grid()
    a[1].set_ylabel("backlash axis position")
    a[1].legend()
    a[1].grid()
    
plt.legend()

plt.legend()
plt.show()

