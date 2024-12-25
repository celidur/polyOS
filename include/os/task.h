#ifndef TASK_H
#define TASK_H

#include <os/config.h>
#include <os/types.h>
#include <os/paging.h>
#include <os/process.h>

struct registers
{
    u32 edi;
    u32 esi;
    u32 ebp;
    u32 ebx;
    u32 edx;
    u32 ecx;
    u32 eax;

    u32 ip;
    u32 cs;
    u32 flags;
    u32 esp;
    u32 ss;
}__attribute__((packed));

struct process;
struct task
{
    page_t *page_directory;
    struct registers regs;

    struct process *process;

    struct task *next;
    struct task *prev;
};

void task_return(struct registers *regs) __attribute__((noreturn));
void user_registers();
int task_page();
void task_run_first_ever_task() __attribute__((noreturn));
int task_switch(struct task *task);
struct task *task_new(struct process *process);
struct task *task_current();
struct task *task_get_next();
int task_free(struct task *task);

struct interrupt_frame;
void task_current_save_state(struct interrupt_frame *frame);
int copy_string_from_task(struct task *task, void *virt, void *phys, int max);
int copy_string_to_task(struct task *task, void* buff, void* virt, u32 size);
void* task_virtual_address_to_physical(struct task* task, void* virtual_address);
void task_next();

#endif
