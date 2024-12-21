[BITS 32]

section .asm

extern interrupt_handler
extern interrupt_handler_error
extern int80h_handler

global idt_load
global enable_interrupts
global disable_interrupts
global are_interrupts_enabled
global int80h_wrapper
global interrupt_pointer_table
global get_cr2

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

%if %1 != 8 && %1 != 10 && %1 != 11 && %1 != 12 && %1 != 13 && %1 != 14
        call interrupt_handler   
%else
        mov eax, esp
        mov eax, [eax] ; get the error code
        push eax
        call interrupt_handler_error
        add esp, 4
%endif
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

are_interrupts_enabled:
    pushfd
    pop eax
    test eax, 0x200
    ret

get_cr2:
    mov eax, cr2
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