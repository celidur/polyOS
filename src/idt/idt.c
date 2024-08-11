#include <os/idt.h>
#include <os/config.h>
#include <os/kernel.h>
#include <os/terminal.h>
#include <os/memory.h>
#include <os/task.h>
#include <os/status.h>
#include <os/io.h>
#include <os/process.h>

static struct idt_desc idt_descriptors[TOTAL_INTERRUPTS];
static struct idtr_desc idtr_descriptor;

extern void* interrupt_pointer_table[TOTAL_INTERRUPTS];

static INTERRUPT_CALLBACK_FUNC interrupt_callbacks[TOTAL_INTERRUPTS];
static INTERRUPT_CALLBACK_FUNC_ERROR interrupt_callbacks_error[TOTAL_INTERRUPTS];
static INT80H_COMMAND int80h_commands[MAX_INT80H_COMMANDS];

extern void idt_load(struct idtr_desc* ptr);
extern void int80h_wrapper();
extern u32 get_cr2();

int idt_register_interrupt_callback(int interrupt, INTERRUPT_CALLBACK_FUNC callback)
{
    if (interrupt < 0 || interrupt >= TOTAL_INTERRUPTS)
    {
        return -EINVARG;
    }
    interrupt_callbacks[interrupt] = callback;
    return ALL_OK;
}

int idt_register_interrupt_callback_error(int interrupt, INTERRUPT_CALLBACK_FUNC_ERROR callback)
{
    if (interrupt < 0 || interrupt >= TOTAL_INTERRUPTS)
    {
        return -EINVARG;
    }
    interrupt_callbacks_error[interrupt] = callback;
    return ALL_OK;
}

static void idt_clock(struct interrupt_frame* frame)
{
    outb(0x20, 0x20);
    task_next();
}

static void idt_page_fault(struct interrupt_frame* frame, u32 code_error)
{
    u32 faulting_address = get_cr2();

    int p = code_error & 0x1;
    int w = (code_error >> 1) & 0x1;
    int u = (code_error >> 2) & 0x1;
    int r = (code_error >> 3) & 0x1;
    int i = (code_error >> 4) & 0x1;
    int pk = (code_error >> 5) & 0x1;
    int ss = (code_error >> 6) & 0x1;
    int SGX = (code_error >> 15) & 0x1;

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

static void idt_general_protection_fault(struct interrupt_frame* frame, u32 code_error)
{
    printf("General protection fault\n");
    int e = code_error & 0x1;
    if (e){
        printf("the exception originated externally to the processor\n");
    }else{
        int Tbl = (code_error >> 1) & 0x3;
        int index = (code_error >> 3) & 0x1FFF;
        switch (Tbl)
        {
        case 0:
            printf("GDT");
            break;
        case 1:
        case 3:
            printf("IDT");
            break;
        case 2:
            printf("LDT");
            break;
        default:
            break;
        }
        printf(" index: 0x%x\n", index);
    }
    kernel_panic("General protection fault");
}

static void idt_handle_exception(){
    process_terminate(task_current()->process);
    task_next();
}

void interrupt_handler(int interrupt,struct interrupt_frame* frame)
{
    kernel_page();
    task_current_save_state(frame);
    if (interrupt_callbacks[interrupt] != 0)
        interrupt_callbacks[interrupt](frame);

    task_page();
    outb(0x20, 0x20);
}

void interrupt_handler_error(u32 error_code, int interrupt,struct interrupt_frame* frame)
{
    kernel_page();
    task_current_save_state(frame);
    if (interrupt_callbacks_error[interrupt] != 0)
        interrupt_callbacks_error[interrupt](frame, error_code);
    task_page();
    outb(0x20, 0x20);
}


static void idt_set(int interrupt_no, void *address)
{
    struct idt_desc *desc = &idt_descriptors[interrupt_no];
    desc->offset_1 = (u16)((u32)address & 0xFFFF);
    desc->offset_2 = (u16)(((u32)address >> 16) & 0xFFFF);
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
    frame->eax = (uint32_t)(res);
    task_current_save_state(frame);
    task_page();
    return res;
}

void int80h_register_command(int command_id, INT80H_COMMAND handler)
{
    if (command_id < 0 || command_id >= MAX_INT80H_COMMANDS)
    {
        kernel_panic("Invalid command id\n");
    }
    if (int80h_commands[command_id] == handler)
        return;

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

    idtr_descriptor.base = (u32)idt_descriptors;

    for (int i = 0; i < TOTAL_INTERRUPTS; i++)
    {
        idt_set(i, interrupt_pointer_table[i]);
    }

    idt_set(0x80, int80h_wrapper);

    for (int i = 0; i < 0x20; i++)
    {
        idt_register_interrupt_callback(i, idt_handle_exception);
        idt_register_interrupt_callback_error(i, idt_handle_exception);
    }

    idt_register_interrupt_callback(0x20, idt_clock);

    idt_register_interrupt_callback_error(0xE, idt_page_fault);
    idt_register_interrupt_callback_error(0xD, idt_general_protection_fault);

    idt_load(&idtr_descriptor);
}

