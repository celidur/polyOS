#include "task.h"
#include "kernel.h"
#include "status.h"
#include "memory/heap/kheap.h"
#include "memory/memory.h"
#include "idt/idt.h"

struct task *current_task = NULL;

struct task *task_tail = NULL;
struct task *task_head = NULL;

int task_init(struct task *task, struct process *process);

struct task *task_current()
{
    return current_task;
}

struct task *task_new(struct process *process)
{
    int res = 0;
    struct task *task = kzalloc(sizeof(struct task));
    if (!task)
    {
        res = -ENOMEM;
        goto out;
    }

    res = task_init(task, process);
    if (res != ALL_OK)
    {
        res = -ENOMEM;
        goto out;
    }

    if (!task_head)
    {
        task_head = task;
        task_tail = task;
        current_task = task;
        goto out;
    }

    task_tail->next = task;
    task->prev = task_tail;
    task_tail = task;

out:
    if (ISERR(res))
    {
        task_free(task);
        return NULL;
    }

    return task;
}

static void task_list_remove(struct task *task)
{
    if (task->prev)
        task->prev->next = task->next;

    if (task->next)
        task->next->prev = task->prev;

    if (task_head == task)
        task_head = task->next;

    if (task_tail == task)
        task_tail = task->prev;

    if (current_task == task)
        current_task = task->next;
}

int task_free(struct task *task)
{
    if (!task)
    {
        return -EINVARG;
    }
    paging_free_4gb(task->page_directory);
    task_list_remove(task);

    kfree(task);
    return 0;
}

int task_init(struct task *task, struct process *process)
{
    memset(task, 0, sizeof(struct task));
    task->page_directory = paging_new_4gb(PAGING_IS_PRESENT | PAGING_ACCESS_FROM_ALL);
    if (!task->page_directory)
    {
        return -EIO;
    }

    task->regs.cs = USER_CODE_SEGMENT;
    task->regs.ip = PROGRAM_VIRTUAL_ADDRESS;
    task->regs.ss = USER_DATA_SEGMENT;
    task->regs.esp = USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START;
    task->process = process;

    return 0;
}

struct task *task_get_next()
{
    if (!current_task)
    {
        return task_head;
    }

    if (!current_task->next)
    {
        return task_head;
    }

    return current_task->next;
}

int task_switch(struct task *task)
{
    if (!task)
    {
        return -EINVARG;
    }

    current_task = task;
    paging_switch(task->page_directory);
    return 0;
}

void task_run_first_ever_task()
{
    if (!current_task)
    {
        kernel_panic("task_run_first_ever_task: No current task exist! \n");
    }

    task_switch(task_head);
    task_return(&task_head->regs);
}

int task_page()
{
    user_registers();
    return task_switch(current_task);
}

void task_save_state(struct task *task, struct interrupt_frame *frame)
{
    task->regs.edi = frame->edi;
    task->regs.esi = frame->esi;
    task->regs.ebp = frame->ebp;
    task->regs.ebx = frame->ebx;
    task->regs.edx = frame->edx;
    task->regs.ecx = frame->ecx;
    task->regs.eax = frame->eax;

    task->regs.ip = frame->ip;
    task->regs.cs = frame->cs;
    task->regs.flags = frame->flags;
    task->regs.esp = frame->esp;
    task->regs.ss = frame->ss;
}

void task_current_save_state(struct interrupt_frame *frame)
{
    if (!current_task)
    {
        kernel_panic("task_current_save_state: No current task to save! \n");
    }

    struct task *task = task_current();
    task_save_state(task, frame);
}