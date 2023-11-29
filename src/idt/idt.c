#include "idt.h"
#include "config.h"
#include "kernel.h"
#include "memory/memory.h"
#include "task/task.h"

#include "io/io.h"

struct idt_desc idt_descriptors[TOTAL_INTERRUPTS];
struct idtr_desc idtr_descriptor;
static ISR80H_COMMAND int80h_commands[MAX_INT80H_COMMANDS];

extern void idt_load(struct idtr_desc *ptr);
extern void int21h();
extern void no_interrupt();
extern void int80h_wrapper();

void idt_zero()
{
    print("Divide by zero error\n");
    while (1)
    {
    }
}

void idt_set(int interrupt_no, void *address)
{
    struct idt_desc *desc = &idt_descriptors[interrupt_no];
    desc->offset_1 = (uint16_t)((uint32_t)address & 0xFFFF);
    desc->offset_2 = (uint16_t)(((uint32_t)address >> 16) & 0xFFFF);
    desc->selector = KERNEL_CODE_SELECTOR;
    desc->zero = 0x00;
    desc->type_attr = 0xEE;
}

void int21h_handler()
{
    print("Keyboard pressed!\n");
    outb(0x20, 0x20);
}

void no_interrupt_handler()
{
    outb(0x20, 0x20);
}

void *int80h_handle_command(int command, struct interrupt_frame *frame)
{
    if (command < 0 || command >= MAX_INT80H_COMMANDS)
        return NULL;

    ISR80H_COMMAND handler = int80h_commands[command];
    if (!handler)
        return NULL;
    return handler(frame);
}

void *int80h_handler(int command, struct interrupt_frame *frame)
{
    void *res = NULL;
    kernel_page();
    task_current_save_state(frame);
    res = int80h_handle_command(command, frame);
    task_page();
    return res;
}

void int80h_register_command(int command_id, ISR80H_COMMAND handler)
{
    if (command_id < 0 || command_id >= MAX_INT80H_COMMANDS)
    {
        kernel_panic("Invalid command id\n");
    }
    if (int80h_commands[command_id])
    {
        kernel_panic("Command already registered\n");
    }
    int80h_commands[command_id] = handler;
}

void idt_init()
{
    memset(idt_descriptors, 0, sizeof(idt_descriptors));

    idtr_descriptor.limit = sizeof(idt_descriptors) - 1;

    idtr_descriptor.base = (uint32_t)idt_descriptors;

    for (int i = 0; i < TOTAL_INTERRUPTS; i++)
    {
        idt_set(i, no_interrupt);
    }

    idt_set(0, idt_zero);

    idt_set(0x21, int21h);

    idt_set(0x80, int80h_wrapper);

    idt_load(&idtr_descriptor);
}
