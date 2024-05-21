from ethercat_esi_generator import *
import os

# use commandline args to input the yaml file
import sys
config_file = sys.argv[1]

# Parse the YAML file
yaml_data = parse_yaml(config_file)
print(yaml_data)
# Extract the orbita types from the YAML data
device_types = [slave['orbita_type'] for slave in yaml_data['slaves']]

# Generate XML for the devices specified in the YAML file
tree = generate_xml(device_types)
# Write the XML to a file
# check if abosulte path is provided for the config file
if os.path.isabs(yaml_data["ethercat"]["esi"]):
    xml_path = yaml_data["ethercat"]["esi"]
else:
    # ethercat/esi contains relative path from the config file
    xml_path = os.path.join(os.path.dirname(config_file), yaml_data["ethercat"]["esi"])
write_xml(tree, xml_path)

print("XML files generated successfully.")