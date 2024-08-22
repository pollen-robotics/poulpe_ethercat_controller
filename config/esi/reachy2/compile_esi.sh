for i in *.xml; do
    siitool $i -m -o ${i%.xml}.bin
done