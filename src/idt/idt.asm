[BITS 32]

section .asm

extern interrupt_handler
extern int80h_handler

global idt_load
global enable_interrupts
global disable_interrupts
global int80h_wrapper
global interrupt_pointer_table
idt_load:
    push ebp
    mov ebp, esp

    mov eax, [ebp+8]
    lidt [eax]

    pop ebp
    ret

int80h_wrapper:
    pushad

    push esp ; push the stack pointer so pointing to the frame structure
    call int80h_handler
    mov dword[tmp_res], eax
    add esp, 4

    popad
    mov eax, [tmp_res]
    iretd

%macro interrupt 1
    global int%1
    int%1:
        ; INTERRUPT FRAME START
        ; ALREADY PUSHED TO US BY THE PROCESSOR UPON ENTRY TO THIS INTERRUPT
        ; uint32_t ip
        ; uint32_t cs;
        ; uint32_t flags
        ; uint32_t sp;
        ; uint32_t ss;
        ; Pushes the general purpose registers to the stack
        pushad
        ; Interrupt frame end
        push esp
        push dword %1
        call interrupt_handler
        add esp, 8
        popad
        iret
%endmacro

%assign i 0
%rep 512
    interrupt i
%assign i i+1
%endrep

enable_interrupts:
    sti
    ret

disable_interrupts:
    cli
    ret

section .data
tmp_res: dd 0

%macro interrupt_array_entry 1
    dd int%1
%endmacro

interrupt_pointer_table:
%assign i 0
%rep 512
    interrupt_array_entry i
%assign i i+1
%endrep