requires  
- `pyesi` : https://github.com/pollen-robotics/pyesi
- `siitool` : https://github.com/synapticon/siitool

To regeneate the `xml` files  (requires `pyesi`)
```bash
python3 -m generate_robot_esi_files
```

to generate the bin files (requires `siitool`)
```bash
sh compile_esi.sh
```