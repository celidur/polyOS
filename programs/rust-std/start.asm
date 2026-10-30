bits 32
section .text
global _start
extern __polyos_std_start
_start:
    mov eax, [esp]
    mov ebx, [esp+4]
    mov ecx, [esp+8]
    and esp, -16
    sub esp, 4
    push ecx
    push ebx
    push eax
    call __polyos_std_start
    ud2
