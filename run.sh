# /bin/bash
sh ./build.sh
qemu-system-x86_64 -display curses -hda ./bin/os.bin
