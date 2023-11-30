[BITS 32]
section .asm

global _start

_start:
    push msg
    mov eax, 1
    int 0x80
    add esp, 4
    jmp $

section .data
msg: db "Hello World from user", 0
