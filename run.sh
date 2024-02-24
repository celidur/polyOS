# /bin/bash
make clean
sh ./build.sh
qemu-system-x86_64 -hda ./bin/os.bin -serial stdio
