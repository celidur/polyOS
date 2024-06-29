# gdb command

## First :

- add-symbol-file ./build/kernelfull.o 0x100000
- target remote | qemu-system-x86_64 -drive format=raw,file=./bin/os.bin -S -gdb stdio -serial file:"log/log_$(date +'%Y%m%d_%H%M%S').txt"


## view assembly code
- layout asm

- layout src

- view registers
- info registers

- stepi execute one instruction
- nexti execute one instruction, but step over function calls
- next execute the current C instruction, but step over function calls

