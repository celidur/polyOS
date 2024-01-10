[BITS 32]

global _start
extern c_start
extern polyos_exit

section .asm

_start:
    call c_start
    call polyos_exit
    ret
