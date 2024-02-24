#ifndef TASK_H
#define TASK_H

#include "config.h"
#include "memory/paging/paging.h"
#include "process.h"

struct registers
{
    uint32_t edi;
    uint32_t esi;
    uint32_t ebp;
    uint32_t ebx;
    uint32_t edx;
    uint32_t ecx;
    uint32_t eax;

    uint32_t ip;
    uint32_t cs;
    uint32_t flags;
    uint32_t esp;
    uint32_t ss;
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

void task_return(struct registers *regs);
void user_registers();
int task_page();
void task_run_first_ever_task();
int task_switch(struct task *task);
struct task *task_new(struct process *process);
struct task *task_current();
struct task *task_get_next();
int task_free(struct task *task);

struct interrupt_frame;
void task_current_save_state(struct interrupt_frame *frame);
int copy_string_from_task(struct task *task, void *virt, void *phys, int max);
void* task_virtual_address_to_physical(struct task* task, void* virtual_address);
void task_next();

#endif
