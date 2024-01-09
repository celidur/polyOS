[BITS 32]
section .asm

global _start

_start:

__loop:
    call getkey
    push eax
    mov eax, 3 ; Command putchar
    int 0x80
    add esp, 4
    jmp __loop

getkey:
    mov eax, 2 ; Command getkey
    int 0x80
    cmp eax, 0
    je getkey
    ret

section .data
msg: db "Hello World from user", 0
