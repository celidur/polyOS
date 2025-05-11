#ifndef PROCESS_H
#define PROCESS_H
#include <os/config.h>
#include <os/types.h>
#include <os/task.h>
#include <os/elfloader.h>

#define PROCESS_FILETYPE_ELF 0
#define PROCESS_FILETYPE_BINARY 1

typedef u8 PROCESS_FILETYPE;

struct process_allocation{
    void* ptr;
    size_t size;
};

struct command_argument{
    char argument[512];
    struct command_argument* next;
};

struct process_argument{
    int argc;
    char** argv;
};

struct process
{
    u16 pid;
    char filename[MAX_PATH];
    struct keyboard_buffer
    {
        char buffer[KEYBOARD_BUFFER_SIZE];
        int head;
        int tail;
    } keyboard;

    struct process_argument arguments;

    struct task *task;

    struct process_allocation allocations[MAX_PROGRAM_ALLOCATIONS];
    PROCESS_FILETYPE filetype;
    union 
    {
       void *ptr;     // physical address of the process
        struct elf_file *elf_file;
    };
    
    void *stack;   // physical address of the stack
    u32 size; // size of the data pointed to by "ptr"
};

// int process_load(const char *filename, struct process **process);
// void *task_get_stack_item(struct task *task, int item);
// int task_page_task(struct task *task);
// struct process *process_current();
// struct process *process_get(int process_id);

// int process_switch(struct process *process);
// int process_load_switch(const char *filename, struct process **process);
// void* process_malloc(struct process* process, size_t size);
// void process_free(struct process* process, void* ptr);

// void process_get_arguments(struct process* process, int* argc, char*** argv);
// int process_inject_arguments(struct process* process, struct command_argument* root_command);
int process_terminate();
#endif