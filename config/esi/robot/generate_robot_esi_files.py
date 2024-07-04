
from pyesi.generator import *


orbitas = {
    "Neck": 3,
    "RightShoulder": 2,
    "RightElbow": 2,
    "RightWrist": 3,
    "LeftShoulder": 2,
    "LeftElbow": 2,
    "LeftWrist": 3,
}



def generate_orbita_esi(name, orbita_type):

    # Generate XML for the master with two slaves
    esi = ESI()
    esi.vendor_id = "0xF3F"
    esi.vendor_name = "Pollen Robotcs SAS"
    esi.group_name = "Pollen PYESI"
    # esi.devices = []

    slave = Device()
    slave.name = f"{name}Orbita{orbita_type}d"

    pdos = PDOs()
    pdos.sm_type = SyncManagerType.BUFFERED
    pdos.address = "1000" 
    pdos.name = "OrbitaIn"
    pdos.entries = [Entry(name="torque_state", type=EntryType.UINT8)]
    for i in range(orbita_type):
        pdos.entries.append(Entry(name="target", type=EntryType.REAL))
        pdos.entries.append(Entry(name="velocity_limit", type=EntryType.REAL))
        pdos.entries.append(Entry(name="torque_limit", type=EntryType.REAL))
    slave.RxPdos.append(pdos)


    pdos = PDOs()
    pdos.sm_type = SyncManagerType.MAILBOX
    pdos.address = "1200" 
    pdos.name = "OrbitaState"
    pdos.entries = [Entry(name="state", type=EntryType.UINT8)]
    pdos.entries.append(Entry(name="type", type=EntryType.UINT8))
    slave.TxPdos.append(pdos)

    pdos = PDOs()
    pdos.sm_type = SyncManagerType.BUFFERED
    pdos.address = "1300" 
    pdos.name = "OrbitaOut"
    pdos.entries = [Entry(name="torque_enabled", type=EntryType.UINT8)]
    for i in range(orbita_type):
        pdos.entries.append(Entry(name="position", type=EntryType.REAL))
        pdos.entries.append(Entry(name="velocity", type=EntryType.REAL))
        pdos.entries.append(Entry(name="torque", type=EntryType.REAL))
        pdos.entries.append(Entry(name="axis_sensor", type=EntryType.REAL))
    slave.TxPdos.append(pdos)

    esi.devices.append(slave)
    tree = esi.to_xml()
    write_xml(tree, slave.name+".xml")
    print("XML file generated successfully: "+slave.name+".xml")

    del esi
    del tree
    del pdos
    del slave


if __name__ == "__main__":
    for name, orbita_type in orbitas.items():
        generate_orbita_esi(name, orbita_type)