section .asm

global idt_load
idt_load:
    push ebp
    mov ebp, esp

    mov eax, [ebp+8]
    lidt [eax]

    pop ebp
    ret