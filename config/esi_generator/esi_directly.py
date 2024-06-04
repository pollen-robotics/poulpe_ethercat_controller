from ethercat_esi_generator import *

# Generate XML for the master with two slaves
# 0: orbita 3d 
# 1: orbita 2d
tree_3d = generate_xml([3,2])
write_xml(tree_3d, "orbitas.xml")

print("XML files generated successfully.")