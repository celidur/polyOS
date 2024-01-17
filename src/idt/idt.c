#include "idt.h"
#include "config.h"
#include "kernel.h"
#include "terminal/terminal.h"
#include "memory/memory.h"
#include "task/task.h"
#include "status.h"
#include "io/io.h"
#include "task/process.h"

static struct idt_desc idt_descriptors[TOTAL_INTERRUPTS];
static struct idtr_desc idtr_descriptor;

extern void* interrupt_pointer_table[TOTAL_INTERRUPTS];

static INTERRUPT_CALLBACK_FUNC interrupt_callbacks[TOTAL_INTERRUPTS];
static INT80H_COMMAND int80h_commands[MAX_INT80H_COMMANDS];

extern void idt_load(struct idtr_desc* ptr);
extern void int80h_wrapper();
extern uint32_t get_cr2();

int idt_register_interrupt_callback(int interrupt, INTERRUPT_CALLBACK_FUNC callback)
{
    if (interrupt < 0 || interrupt >= TOTAL_INTERRUPTS)
    {
        return -EINVARG;
    }
    interrupt_callbacks[interrupt] = callback;
    return ALL_OK;
}

static void idt_clock(struct interrupt_frame* frame)
{
    outb(0x20, 0x20);
    task_next();
}

static void idt_page_fault(struct interrupt_frame* frame)
{
    uint32_t faulting_address = get_cr2();

    uint32_t error_code_ptr = ((uint32_t) (&frame->edi)) - 8 ;
    uint32_t error_code = (*(uint32_t*) error_code_ptr);

    int p = error_code & 0x1;
    int w = (error_code >> 1) & 0x1;
    int u = (error_code >> 2) & 0x1;
    int r = (error_code >> 3) & 0x1;
    int i = (error_code >> 4) & 0x1;
    int pk = (error_code >> 5) & 0x1;
    int ss = (error_code >> 6) & 0x1;
    int SGX = (error_code >> 15) & 0x1;

    printf("Page fault( ");
    if (p)
        printf("protection violation ");
    if (w)
        printf("write ");
    else
        printf("read ");
    if (u)
        printf("user ");
    else
        printf("supervisor ");
    if (r)
        printf("reserved ");
    if (i)
        printf("instruction fetch ");
    if (pk)
        printf("protection key violation ");
    if (ss)
        printf("shadow stack ");
    if (SGX)
        printf("SGX ");
    printf(") at 0x%x\n", faulting_address);

    struct registers* regs = &task_current()->regs;
    printf("Registers:\n");
    printf("edi: 0x%x\n", regs->edi);
    printf("esi: 0x%x\n", regs->esi);
    printf("ebp: 0x%x\n", regs->ebp);
    printf("ebx: 0x%x\n", regs->ebx);
    printf("edx: 0x%x\n", regs->edx);
    printf("ecx: 0x%x\n", regs->ecx);
    printf("eax: 0x%x\n", regs->eax);
    printf("ip: 0x%x\n", regs->ip);
    printf("cs: 0x%x\n", regs->cs);
    printf("flags: 0x%x\n", regs->flags);
    printf("esp: 0x%x\n", regs->esp);
    printf("ss: 0x%x\n", regs->ss);
    
    kernel_panic("Page fault");
}

static void idt_handle_exception(){
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


static void idt_set(int interrupt_no, void *address)
{
    struct idt_desc *desc = &idt_descriptors[interrupt_no];
    desc->offset_1 = (uint16_t)((uint32_t)address & 0xFFFF);
    desc->offset_2 = (uint16_t)(((uint32_t)address >> 16) & 0xFFFF);
    desc->selector = KERNEL_CODE_SELECTOR;
    desc->zero = 0x00;
    desc->type_attr = 0xEE;
}

static void *int80h_handle_command(struct interrupt_frame *frame)
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

    idt_register_interrupt_callback(0xE, idt_page_fault);

    idt_load(&idtr_descriptor);
}

