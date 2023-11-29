# /bin/bash
make clean
sh ./build.sh
qemu-system-x86_64 -display curses -hda ./bin/os.bin

# gdb command
# target remote | qemu-system-i386 -hda ./os.bin -S -gdb stdio

# view assembly code
# layout asm

# view registers
# info registers
