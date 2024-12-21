#ifndef IDT_H
#define IDT_H

#include <os/types.h>
#include <os/terminal.h>

struct idt_desc
{
    u16 offset_1;
    u16 selector;
    u8 zero;
    u8 type_attr;
    u16 offset_2;
} __attribute__((packed));

struct idtr_desc
{
    u16 limit;
    u32 base;
} __attribute__((packed));

struct interrupt_frame
{
    u32 edi;
    u32 esi;
    u32 ebp;
    u32 reserved;
    u32 ebx; 
    u32 edx;
    u32 ecx;
    u32 eax;

    u32 ip; // instruction pointer
    u32 cs;
    u32 flags;
    u32 esp;
    u32 ss;
} __attribute__((packed));

void idt_init();

void enable_interrupts();
void disable_interrupts();
u32 are_interrupts_enabled();

struct interrupt_frame;
typedef void *(*INT80H_COMMAND)(struct interrupt_frame *frame);
typedef void(*INTERRUPT_CALLBACK_FUNC)(struct interrupt_frame *frame);
typedef void(*INTERRUPT_CALLBACK_FUNC_ERROR)(struct interrupt_frame *frame, u32 code_error);
void int80h_register_command(int command_id, INT80H_COMMAND handler);
int idt_register_interrupt_callback(int interrupt, INTERRUPT_CALLBACK_FUNC callback);
int idt_register_interrupt_callback_error(int interrupt, INTERRUPT_CALLBACK_FUNC_ERROR callback);


#endif