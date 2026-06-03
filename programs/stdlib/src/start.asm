[BITS 32]

global _start
extern c_start
extern _exit

section .asm

_start:
    call c_start
    push 0
    call _exit
    add esp, 4
    ret
