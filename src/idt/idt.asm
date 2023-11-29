[BITS 32]

section .asm

extern int21h_handler
extern no_interrupt_handler
extern int80h_handler

global int21h
global no_interrupt
global idt_load
global enable_interrupts
global disable_interrupts
global int80h_wrapper
idt_load:
    push ebp
    mov ebp, esp

    mov eax, [ebp+8]
    lidt [eax]

    pop ebp
    ret

int21h:
    pushad
    call int21h_handler
    popad
    iret

int80h_wrapper:
    pushad

    push esp ; push the stack pointer so pointing to the frame structure
    push eax
    call int80h_handler
    mov [tmp_res], eax
    add esp, 8

    popad
    mov eax, [tmp_res]
    iretd

no_interrupt:
    pushad
    call no_interrupt_handler
    popad
    iret

enable_interrupts:
    sti
    ret

disable_interrupts:
    cli
    ret

section .data
tmp_res: dd 0