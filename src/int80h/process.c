#include <os/int80/process.h>
#include <os/process.h>
#include <os/task.h>
#include <os/string.h>
#include <os/status.h>
#include <os/config.h>
#include <os/kernel.h>

void* int80h_command6_process_load_start(struct interrupt_frame *frame){
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

void* int80h_command7_invoke_system_command(struct interrupt_frame *frame){
    struct command_argument* args = task_virtual_address_to_physical(task_current(), task_get_stack_item(task_current(), 0));
    if (!args || strnlen(args[0].argument, 2) == 0){
        return (void*)-EINVARG;
    }

    struct command_argument* root_command = &args[0];
    const char* program_name = root_command->argument;
    char path[MAX_PATH];
    strcpy(path, "0:/");
    strncpy(path+3, program_name, sizeof(path)-3);

    struct process* process = NULL;
    int res = process_load_switch(path, &process);
    if (res<0){
        return (void*)res;
    }

    res = process_inject_arguments(process, root_command);
    if (res<0){
        return (void*)res;
    }
    task_switch(process->task);
    task_return(&process->task->regs);

    return NULL;
}

void* int80h_command8_get_program_arguments(struct interrupt_frame *frame){
    struct process* process = task_current()->process;
    struct process_argument* args = task_virtual_address_to_physical(task_current(), task_get_stack_item(task_current(), 0));
    process_get_arguments(process, &args->argc, &args->argv);
    return NULL;
}

void* int80h_command9_exit(struct interrupt_frame *frame){
    struct process* process = task_current()->process;
    process_terminate(process);
    task_next();
    kernel_panic("No more tasks to run\n");
    return NULL;
}