#include "process.h"
#include "task/process.h"
#include "task/task.h"
#include "string/string.h"
#include "status.h"
#include "config.h"

#include "kernel.h"

void* int80_command6_process_load_start(struct interrupt_frame *frame){
    void * file_user_ptr = (void*)task_get_stack_item(task_current(), 0);
    char filename[MAX_PATH];
    int res = copy_string_from_task(task_current(), file_user_ptr,filename, sizeof(filename));
    if (res != ALL_OK){
        return (void*)res;
    }
    char path[MAX_PATH];
    strcpy(path, "0:/");
    strcpy(path+3, filename);

    struct process* process = NULL;
    res = process_load_switch(path, &process);
    if (res != ALL_OK){
        return (void*)res;
    }

    task_switch(process->task);
    task_return(&process->task->regs);

    return NULL;
}