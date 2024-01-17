#ifndef IDT_H
#define IDT_H

#include <stdint.h>
#include "terminal/terminal.h"

struct idt_desc
{
    uint16_t offset_1;
    uint16_t selector;
    uint8_t zero;
    uint8_t type_attr;
    uint16_t offset_2;
} __attribute__((packed));

struct idtr_desc
{
    uint16_t limit;
    uint32_t base;
} __attribute__((packed));

struct interrupt_frame
{
    uint32_t edi;
    uint32_t esi;
    uint32_t ebp;
    uint32_t reserved;
    uint32_t ebx; 
    uint32_t edx;
    uint32_t ecx;
    uint32_t eax;

    uint32_t ip; // instruction pointer
    uint32_t cs;
    uint32_t flags;
    uint32_t esp;
    uint32_t ss;
} __attribute__((packed));

void idt_init();

void enable_interrupts();
void disable_interrupts();

struct interrupt_frame;
typedef void *(*INT80H_COMMAND)(struct interrupt_frame *frame);
typedef void(*INTERRUPT_CALLBACK_FUNC)(struct interrupt_frame *frame);
void int80h_register_command(int command_id, INT80H_COMMAND handler);
int idt_register_interrupt_callback(int interrupt, INTERRUPT_CALLBACK_FUNC callback);


#endif