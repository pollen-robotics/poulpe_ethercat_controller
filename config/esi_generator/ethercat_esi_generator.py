import xml.etree.ElementTree as ET
from xml.dom import minidom
import yaml

def create_vendor():
    vendor = ET.Element("Vendor")
    ET.SubElement(vendor, "Id").text = "#xF3F" #pollen vendor id
    ET.SubElement(vendor, "Name").text = "Pollen Robotics SAS"
    ET.SubElement(vendor, "ImageData16x14").text = (
        "424dd6020000000000003600000028000000100000000e0000000100180000000000a0020000c40e0000c40e000000000000000000004cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb1224cb122ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff241cedffffff241cedffffff241ced241cedffffffffffffffffff241ced241ced241cedffffff241cedffffffffffff241cedffffff241cedffffff241cedffffff241cedffffff241cedffffff241cedffffffffffff241cedffffffffffff241cedffffff241cedffffff241cedffffff241cedffffff241cedffffff241cedffffffffffff241cedffffffffffff241cedffffff241cedffffff241cedffffff241cedffffffffffff241cedffffffffffffffffff241cedffffffffffff241ced241ced241cedffffff241ced241cedffffffffffff241cedffffff241cedffffffffffff241cedffffffffffff241cedffffff241cedffffff241cedffffff241cedffffff241cedffffff241cedffffffffffff241cedffffffffffff241cedffffff241cedffffff241cedffffff241cedffffffffffff241cedffffffffffffffffff241cedffffffffffff241cedffffff241cedffffff241cedffffff241cedffffffffffffffffffffffffffffffffffff241cedffffffffffffffffff241cedffffffffffffffffff241cedffffffffffffffffffffffffffffffffffff241ced241ced241cedffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    )
    return vendor

def create_group():
    group = ET.Element("Group", SortOrder="0")
    ET.SubElement(group, "Type").text = "SSC_Device"
    ET.SubElement(group, "Name", LcId="1033").text = "EasyCAT"
    ET.SubElement(group, "ImageData16x14").text = (
      "424dd6020000000000003600000028000000100000000e0000000100180000000000a0020000c40e0000c40e00000000000000000000241ced241ced241ced241cedffffff241cedffffffffffffffffff241cedffffffffffffffffff241cedffffffffffff241cedffffffffffffffffffffffff241cedffffffffffffffffff241cedffffffffffffffffff241cedffffffffffff241cedffffffffffffffffffffffff241ced241ced241ced241ced241cedffffffffffffffffff241cedffffffffffff241cedffffffffffffffffffffffff241cedffffffffffffffffff241cedffffffffffffffffff241cedffffffffffff241cedffffffffffffffffffffffff241ced241cedffffff241ced241cedffffff241cedffffff241cedffffff241ced241ced241ced241ced241cedffffffffffff241ced241ced241cedffffffffffff241ced241ced241ced241ced241cedffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff241ced241ced241cedffffffffffff241ced241ced241cedffffff241ced241ced241cedffffff241ced241ced241ced241cedffffffffffffffffff241cedffffffffffff241cedffffffffffffffffff241cedffffffffffffffffff241ced241cedffffffffffffffffffffffff241ced241cedffffffffffff241ced241ced241cedffffff241ced241ced241ced241ced241cedffffffffffffffffffffffffffffff241cedffffff241cedffffffffffffffffff241cedffffff241ced241cedffffffffffffffffffffffff241ced241ced241cedffffff241ced241ced241cedffffff241cedffffff241ced241cedffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff241ced241ced241cedffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    )
    return group

def create_device(device_type):
    device = ET.Element("Device", Physics="YY")
    device.append(ET.Comment(f"Orbita {device_type}D Device"))
    ET.SubElement(device, "Type", ProductCode="#x1", RevisionNo="#x1", CheckRevisionNo="EQ_OR_G").text = f'Orbita{device_type}d'
    ET.SubElement(device, "Name", LcId="1033").text = F"Orbita{device_type}d"
    ET.SubElement(device, "GroupType").text = "SSC_Device"
    for fmmu in ["Orbita", "MotorsIn", "MotorsOut"]:
        ET.SubElement(device, "Fmmu").text = fmmu

    for sm, start_address, control_byte in [
        ("MotorsIn", "#x1000", "#x64"),
        ("Orbita", "#x1200", "#x20"),
        ("MotorsOut", "#x1300", "#x22")
    ]:
        ET.SubElement(device, "Sm", StartAddress=start_address, ControlByte=control_byte, Enable="1").text = sm

    device.append(ET.Comment("MotorIn PDOs" ))
    rxpdo = ET.SubElement(device, "RxPdo", Fixed="1", Mandatory="1", Sm="0")
    ET.SubElement(rxpdo, "Index").text = "#x1600"
    ET.SubElement(rxpdo, "Name").text = "MotorIn"

    pdos1 = [("#x10", "1", "8", "torque_state", "UINT8")]
    for i in range(device_type):
      pdos1.append((f"#x1{i+1}", "1", "32", "target", "REAL"))
      pdos1.append((f"#x1{i+1}", "2", "32", "velocity_limit", "REAL"))
      pdos1.append((f"#x1{i+1}", "3", "32", "torque_limit", "REAL"))

    for index, subindex, bitlen, name, datatype in pdos1:
        entry = ET.SubElement(rxpdo, "Entry")
        ET.SubElement(entry, "Index").text = index
        ET.SubElement(entry, "SubIndex").text = subindex
        ET.SubElement(entry, "BitLen").text = bitlen
        ET.SubElement(entry, "Name").text = name
        ET.SubElement(entry, "DataType").text = datatype


    device.append(ET.Comment("Orbita State PDOs" ))
    txpdo = ET.SubElement(device, "TxPdo", Fixed="1", Mandatory="1", Sm="1")    
    ET.SubElement(txpdo, "Index").text = "#x1a00"
    ET.SubElement(txpdo, "Name").text = "Orbita"

    pdos2 = [("#x20", "0", "8", "state", "UINT8"),
            ("#x20", "1", "8", "type", "UINT8")]

    for index, subindex, bitlen, name, datatype in pdos2:
        entry = ET.SubElement(txpdo, "Entry")
        ET.SubElement(entry, "Index").text = index
        ET.SubElement(entry, "SubIndex").text = subindex
        ET.SubElement(entry, "BitLen").text = bitlen
        ET.SubElement(entry, "Name").text = name
        ET.SubElement(entry, "DataType").text = datatype


    device.append(ET.Comment("MotorOut PDOs" ))
    txpdo = ET.SubElement(device, "TxPdo", Fixed="1", Mandatory="1", Sm="2")    
    ET.SubElement(txpdo, "Index").text = "#x1a01"
    ET.SubElement(txpdo, "Name").text = "MotorOut"

    pdos3 = [("#x30", "0", "8", "torque_enabled", "UINT8")]
    for i in range(device_type):
        pdos3.append((f"#x3{i+1}", "1", "32", "position", "REAL"))
        pdos3.append((f"#x3{i+1}", "2", "32", "velocity", "REAL"))
        pdos3.append((f"#x3{i+1}", "3", "32", "torque", "REAL"))
        pdos3.append((f"#x3{i+1}", "4", "32", "axis_sensor", "REAL"))

    for index, subindex, bitlen, name, datatype in pdos3:
        entry = ET.SubElement(txpdo, "Entry")
        ET.SubElement(entry, "Index").text = index
        ET.SubElement(entry, "SubIndex").text = subindex
        ET.SubElement(entry, "BitLen").text = bitlen
        ET.SubElement(entry, "Name").text = name
        ET.SubElement(entry, "DataType").text = datatype

    device.append(generate_sync_manager())
    device.append(generate_ln9252_config())

    return device


def generate_sync_manager():
    sync_manager = ET.Element("Dc")


    op_mode1 = ET.SubElement(sync_manager, "OpMode")
    ET.SubElement(op_mode1, "Name").text = "SM_Sync or Async"
    ET.SubElement(op_mode1, "Desc").text = "SM_Sync or Async"
    ET.SubElement(op_mode1, "AssignActivate").text = "#x0000"


    op_mode = ET.SubElement(sync_manager, "OpMode")
    ET.SubElement(op_mode, "Name").text = "DC_Sync"
    ET.SubElement(op_mode, "Desc").text = "DC_Sync"
    ET.SubElement(op_mode, "AssignActivate").text = "#x300"
    ET.SubElement(op_mode, "CycleTimeSync0", Factor="1").text = "0"
    ET.SubElement(op_mode, "ShiftTimeSync0").text = "2000200000"
    return sync_manager
 
def generate_ln9252_config():
    eeprom = ET.Element("Eeprom")
    ET.SubElement(eeprom, "ByteSize").text = "4096"
    ET.SubElement(eeprom, "ConfigData").text = "8003006EFF00FF000000"
    eeprom.append(ET.Comment("0x140   0x80 PDI type LAN9252 Spi  "))
    eeprom.append(ET.Comment("0x141   0x03 device emulation     "))
    eeprom.append(ET.Comment("        enhanced link detection        "))
    eeprom.append(ET.Comment("0x150   0x00 not used for LAN9252 Spi  "))
    eeprom.append(ET.Comment("0x151   0x6E map Sync0 to AL event     "))
    eeprom.append(ET.Comment("        Sync0/Latch0 assigned to Sync0 "))
    eeprom.append(ET.Comment("        Sync1/Latch1 assigned to Sync1 "))
    eeprom.append(ET.Comment("        Sync0/1 push/pull active high  "))
    eeprom.append(ET.Comment("0x982-3 0x00FF Sync0/1 lenght = 2.5uS  "))
    eeprom.append(ET.Comment("0x152   0xFF all GPIO set to out       "))
    eeprom.append(ET.Comment("0x153   0x00 reserved                  "))
    eeprom.append(ET.Comment("0x12-13 0x0000 alias address           "))
    return eeprom

def generate_xml(device_types):
    root = ET.Element("EtherCATInfo")
    root.set('xmlns:xsi',"http://www.w3.org/2001/XMLSchema-instance")
    root.set('xsi:noNamespaceSchemaLocation',"EtherCATInfo.xsd")
    root.set("Version", "1.6")

    
    root.append(create_vendor())

    description = ET.Element("Descriptions")
    groups = ET.Element("Groups")
    groups.append(create_group())
    description.append(groups)

    devices = ET.Element("Devices")
    for device_type in device_types:
      devices.append(create_device(device_type))
    description.append(devices)

    root.append(description)

    tree = ET.ElementTree(root)
    return tree

def prettify_xml(tree):
    raw_xml = ET.tostring(tree.getroot(), 'utf-8')
    parsed = minidom.parseString(raw_xml)
    return parsed.toprettyxml(indent="  ")


def write_xml(tree, filename):
  pretty_xml = prettify_xml(tree)
  with open(filename, "w") as f:
      f.write(pretty_xml)

def parse_yaml(file_path):
    with open(file_path, 'r') as file:
        content = file.read().replace("!Poulpe", "")
        return yaml.safe_load(content)

# main code
if __name__ == "__main__":

  # Generate XML for Orbita 2D
  tree_2d = generate_xml([2])
  write_xml(tree_2d, "orbita2d.xml")

  # Generate XML for Orbita 3D with additional entries
  tree_3d = generate_xml([3])
  write_xml(tree_3d, "orbita3d.xml")

  # Generate XML for Orbita 3D with additional entries
  tree_3d = generate_xml([3,2])
  write_xml(tree_3d, "orbitas.xml")

  print("XML files generated successfully.")