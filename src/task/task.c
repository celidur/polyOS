#include <os/task.h>
#include <os/kernel.h>
#include <os/status.h>
#include <os/kheap.h>
#include <os/memory.h>
#include <os/idt.h>
#include <os/paging.h>
#include <os/string.h>
#include <os/elfloader.h>

// static struct task *current_task = NULL;

// static struct task *task_tail = NULL;
// static struct task *task_head = NULL;

// int task_init(struct task *task, struct process *process);

// struct task *task_current()
// {
//     return current_task;
// }

// struct task *task_new(struct process *process)
// {
//     int res = 0;
//     struct task *task = kzalloc(sizeof(struct task));
//     if (!task)
//     {
//         res = -ENOMEM;
//         goto out;
//     }

//     res = task_init(task, process);
//     if (res != ALL_OK)
//     {
//         res = -ENOMEM;
//         goto out;
//     }

//     if (!task_head)
//     {
//         task_head = task;
//         task_tail = task;
//         current_task = task;
//         goto out;
//     }

//     task_tail->next = task;
//     task->prev = task_tail;
//     task_tail = task;

// out:
//     if (ISERR(res))
//     {
//         task_free(task);
//         return NULL;
//     }

//     return task;
// }

// static void task_list_remove(struct task *task)
// {
//     if (task->prev)
//         task->prev->next = task->next;

//     if (task->next)
//         task->next->prev = task->prev;

//     if (task_head == task)
//         task_head = task->next;

//     if (task_tail == task)
//         task_tail = task->prev;

//     if (current_task == task)
//         current_task = task->next;
// }

// int task_free(struct task *task)
// {
//     if (!task)
//     {
//         return -EINVARG;
//     }

//     task_list_remove(task);
//     paging_free_4gb(task->page_directory);

//     kfree(task);
//     return 0;
// }

// int task_init(struct task *task, struct process *process)
// {
//     memset(task, 0, sizeof(struct task));
//     task->page_directory = paging_new_4gb(PAGING_IS_PRESENT);
//     if (!task->page_directory)
//     {
//         return -EIO;
//     }

//     task->regs.cs = USER_CODE_SEGMENT;
//     task->regs.ip = PROGRAM_VIRTUAL_ADDRESS;
//     if (process->filetype == PROCESS_FILETYPE_ELF)
//     {
//         task->regs.ip = elf_header(process->elf_file)->e_entry;
//     }
//     task->regs.ss = USER_DATA_SEGMENT;
//     task->regs.esp = USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START;
//     task->process = process;

//     return 0;
// }

// struct task *task_get_next()
// {
//     if (!current_task)
//     {
//         return task_head;
//     }

//     if (!current_task->next)
//     {
//         return task_head;
//     }

//     return current_task->next;
// }

// int task_switch(struct task *task)
// {
//     if (!task)
//     {
//         return -EINVARG;
//     }

//     current_task = task;
//     paging_switch(task->page_directory);
//     return 0;
// }

// void task_run_first_ever_task()
// {
//     if (!current_task)
//     {
//         kernel_panic("task_run_first_ever_task: No current task exist! \n");
//     }

//     task_switch(task_head);
//     task_return(&task_head->regs);
// }

// int task_page()
// {
//     if (!current_task)
//     {
//         return -EINVARG;
//     }
//     user_registers();
//     return task_switch(current_task);
// }

// void task_save_state(struct task *task, struct interrupt_frame *frame)
// {
//     task->regs.edi = frame->edi;
//     task->regs.esi = frame->esi;
//     task->regs.ebp = frame->ebp;
//     task->regs.ebx = frame->ebx;
//     task->regs.edx = frame->edx;
//     task->regs.ecx = frame->ecx;
//     task->regs.eax = frame->eax;

//     task->regs.ip = frame->ip;
//     task->regs.cs = frame->cs;
//     task->regs.flags = frame->flags;
//     task->regs.esp = frame->esp;
//     task->regs.ss = frame->ss;
// }

// void task_current_save_state(struct interrupt_frame *frame)
// {
//     if (!current_task)
//     {
//         // kernel_panic("task_current_save_state: No current task to save! \n");
//         return;
//     }

//     struct task *task = task_current();
//     task_save_state(task, frame);
// }

int copy_string_from_task(uint32_t *directory, void *virt, void *phys, int max)
{
    if (max >= PAGING_PAGE_SIZE)
        return -EINVARG;

    char *buffer = kpalloc(max);
    if (!buffer)
        return -ENOMEM;

    uint32_t old_entry = paging_get(directory, buffer);

    paging_map(directory, buffer, buffer, PAGING_IS_PRESENT | PAGING_ACCESS_FROM_ALL | PAGING_IS_WRITABLE);
    paging_switch(directory);
    strncpy(buffer, virt, max);
    kernel_page();

    if (paging_set(directory, buffer, old_entry) < 0)
    {
        kfree(buffer);
        return -EIO;
    }

    strncpy(phys, buffer, max);
    kfree(buffer);
    return 0;
}

int copy_string_to_task(u32 *directory, void* buff, void* virt, u32 size)
{
    u32 size_remaining = size;
    u32 size_to_copy = 0;
    u32 offset = 0;
    u32 old_entry = 0;

    while(size_remaining > 0){
        size_to_copy = size_remaining > PAGING_PAGE_SIZE ? PAGING_PAGE_SIZE : size_remaining;
        void *ptr = (void* )((u32)(buff) + offset);

        old_entry = paging_get(directory, paging_align_to_lower_page(ptr));
        paging_map(directory, ptr, ptr, PAGING_IS_PRESENT | PAGING_ACCESS_FROM_ALL | PAGING_IS_WRITABLE);
        paging_switch(directory);
        strncpy(virt, ptr, size_to_copy);
        kernel_page();

        if (paging_set(directory, paging_align_to_lower_page(ptr), old_entry) < 0)
        {
            return -EIO;
        }

        size_remaining -= size_to_copy;
        offset += size_to_copy;
    }
    return 0;
}

void* task_virtual_address_to_physical(u32 *directory, void* virtual_address){
    return paging_get_physical_address(directory, virtual_address);
}

// void task_next(){
//     struct task* next = task_get_next();
//     if (next){
//         task_switch(next);
//         task_return(&next->regs);
//     }
//     return;
// }