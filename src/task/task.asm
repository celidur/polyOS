[BITS 32]
section .asm

global user_registers
global restore_general_registers
global task_return

user_registers:
    mov ax, 0x23
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    ret

restore_general_registers:
    push ebp
    mov ebp, esp
    mov ebx, [ebp + 8]
    mov edi, [ebx]
    mov esi, [ebx + 4]
    mov ebp, [ebx + 8]
    mov edx, [ebx + 16]
    mov ecx, [ebx + 20]
    mov eax, [ebx + 24]
    mov ebx, [ebx + 12]
    add esp, 4
    ret

task_return:
    mov ebp, esp
    mov ebx, [ebp + 4]

    push dword [ebx + 44] ; push the data/stack selector
    push dword [ebx + 40] ; push stack pointer

    ; Push the flags
    mov eax, [ebx+36]
    or eax, 0x200

    push eax

    push dword [ebx + 32] ; push code segment

    push dword [ebx + 28] ; push instruction pointer

    mov ax, [ebx + 44] ; setup data segment
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    push dword [ebp + 4]
    call restore_general_registers
    add esp, 4

    iretd