# /bin/bash
make clean
sh ./build.sh
qemu-system-x86_64 -curses -hda ./bin/os.bin
