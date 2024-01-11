#include "idt.h"
#include "config.h"
#include "kernel.h"
#include "memory/memory.h"
#include "task/task.h"
#include "status.h"
#include "io/io.h"
#include "task/process.h"

struct idt_desc idt_descriptors[TOTAL_INTERRUPTS];
struct idtr_desc idtr_descriptor;

extern void* interrupt_pointer_table[TOTAL_INTERRUPTS];

static INTERRUPT_CALLBACK_FUNC interrupt_callbacks[TOTAL_INTERRUPTS];

static INT80H_COMMAND int80h_commands[MAX_INT80H_COMMANDS];

extern void idt_load(struct idtr_desc* ptr);
extern void int80h_wrapper();


int idt_register_interrupt_callback(int interrupt, INTERRUPT_CALLBACK_FUNC callback)
{
    if (interrupt < 0 || interrupt >= TOTAL_INTERRUPTS)
    {
        return -EINVARG;
    }
    interrupt_callbacks[interrupt] = callback;
    return ALL_OK;
}

void idt_clock()
{
    outb(0x20, 0x20);
    // task_next();
}

void idt_handle_exception(){
    process_terminate(task_current()->process);
    task_next();
}

void interrupt_handler(int interrupt, struct interrupt_frame* frame)
{
    kernel_page();
    if (interrupt_callbacks[interrupt] != 0)
    {
        task_current_save_state(frame);
        interrupt_callbacks[interrupt](frame);
    }

    task_page();
    outb(0x20, 0x20);
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

void *int80h_handle_command(struct interrupt_frame *frame)
{
    int command = frame->eax;
    if (command < 0 || command >= MAX_INT80H_COMMANDS)
        return NULL;

    INT80H_COMMAND handler = int80h_commands[command];
    if (!handler)
        return NULL;
    return handler(frame);
}

void *int80h_handler(struct interrupt_frame *frame)
{
    void *res = NULL;
    kernel_page();
    task_current_save_state(frame);
    res = int80h_handle_command(frame);
    task_page();
    return res;
}

void int80h_register_command(int command_id, INT80H_COMMAND handler)
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
        idt_set(i, interrupt_pointer_table[i]);
    }

    idt_set(0x80, int80h_wrapper);

    for (int i = 0; i < 0x20; i++)
    {
        idt_register_interrupt_callback(i, idt_handle_exception);
    }

    idt_register_interrupt_callback(0x20, idt_clock);

    idt_load(&idtr_descriptor);
}

