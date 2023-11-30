#include "process.h"
#include "memory/memory.h"
#include "config.h"
#include "status.h"
#include "task/task.h"
#include "string/string.h"
#include "fs/file.h"
#include "kernel.h"
#include "memory/paging/paging.h"
#include "memory/heap/kheap.h"

struct process *current_process = NULL;

static struct process *process[MAX_PROCESS] = {NULL};

static void process_init(struct process *process)
{
    memset(process, 0, sizeof(struct process));
}

struct process *process_current()
{
    return current_process;
}

struct process *process_get(int process_id)
{
    if (process_id < 0 || process_id >= MAX_PROCESS)
    {
        return NULL;
    }
    return process[process_id];
}

static int process_load_binary(const char *filename, struct process *process)
{
    int res = 0;
    int fd = fopen(filename, "r");
    if (!fd)
    {
        res = -EIO;
        goto out;
    }

    struct file_stat stat;
    res = fstat(fd, &stat);
    if (res != ALL_OK)
    {
        goto out;
    }

    void *program_data_ptr = kzalloc(stat.size);
    if (!program_data_ptr)
    {
        res = -ENOMEM;
        goto out;
    }

    if (fread(program_data_ptr, stat.size, 1, fd) != 1)
    {
        res = -EIO;
        goto out;
    }

    process->ptr = program_data_ptr;
    process->size = stat.size;

out:
    fclose(fd);
    return res;
}

static int process_load_data(const char *filename, struct process *process)
{
    return process_load_binary(filename, process);
}

int process_map_binary(struct process *process)
{
    paging_map_to(process->task->page_directory, (void *)PROGRAM_VIRTUAL_ADDRESS, process->ptr, paging_align_address(process->ptr + process->size), PAGING_IS_PRESENT | PAGING_IS_WRITABLE | PAGING_ACCESS_FROM_ALL);
    paging_map_to(process->task->page_directory, (void *)USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END, process->stack, paging_align_address(process->stack + USER_PROGRAM_STACK_SIZE), PAGING_IS_PRESENT | PAGING_IS_WRITABLE | PAGING_ACCESS_FROM_ALL);
    return 0;
}

int process_load_for_slot(const char *filename, struct process **process, int process_slot)
{
    int res = 0;
    struct task *task = NULL;
    struct process *_process = NULL;
    void *stack_ptr = NULL;

    if (process_get(process_slot))
    {
        res = -EINVARG;
        goto out;
    }

    _process = kzalloc(sizeof(struct process));
    if (!_process)
    {
        res = -ENOMEM;
        goto out;
    }

    process_init(_process);
    res = process_load_data(filename, _process);
    if (res != ALL_OK)
    {
        goto out;
    }

    stack_ptr = kzalloc(USER_PROGRAM_STACK_SIZE);
    if (!stack_ptr)
    {
        res = -ENOMEM;
        goto out;
    }

    strncpy(_process->filename, filename, sizeof(_process->filename));
    _process->stack = stack_ptr;
    _process->pid = process_slot;

    // create task
    task = task_new(_process);
    if (!task)
    {
        res = -EIO;
        goto out;
    }

    _process->task = task;
    res = process_map_binary(_process);
    if (res != ALL_OK)
    {
        goto out;
    }

    *process = _process;
    process[process_slot] = _process;

out:
    if (ISERR(res))
    {
        if (task)
            task_free(task);
        if (stack_ptr)
            kfree(stack_ptr);
        if (_process)
            kfree(_process);
    }
    return res;
}

int process_get_free_slot()
{
    for (int i = 0; i < MAX_PROCESS; i++)
    {
        if (!process[i])
        {
            return i;
        }
    }
    return -EISTKN;
}

int process_load(const char *filename, struct process **process)
{
    int process_slot = process_get_free_slot();
    if (process_slot < 0)
        return -EISTKN;

    return process_load_for_slot(filename, process, process_slot);
}

int task_page_task(struct task *task)
{
    user_registers();
    paging_switch(task->page_directory);
    return 0;
}

void *task_get_stack_item(struct task *task, int item)
{
    void *result = NULL;
    uint32_t *stack = (uint32_t *)task->regs.esp;
    task_page_task(task);
    result = (void *)stack[item];
    kernel_page();
    return result;
}