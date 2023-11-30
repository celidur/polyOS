# /bin/bash
make clean
sh ./build.sh
qemu-system-x86_64 -display curses -hda ./bin/os.bin

# gdb command
# add-symbol-file ../build/kernelfull.o 0x100000
# target remote | qemu-system-i386 -hda ./os.bin -S -gdb stdio

# view assembly code
# layout asm

# layout code

# view registers
# info registers

# stepi execute one instruction
# nexti execute one instruction, but step over function calls
# next execute the current C instruction, but step over function calls
