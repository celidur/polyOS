#ifndef PROCESS_H
#define PROCESS_H
#include <stdint.h>
#include <stdbool.h>
#include "task.h"
#include "config.h"
#include "loader/formats/elfloader.h"

#define PROCESS_FILETYPE_ELF 0
#define PROCESS_FILETYPE_BINARY 1

typedef unsigned char PROCESS_FILETYPE;
struct process
{
    uint16_t pid;
    char filename[MAX_PATH];
    struct keyboard_buffer
    {
        char buffer[KEYBOARD_BUFFER_SIZE];
        int head;
        int tail;
    } keyboard;

    struct task *task;

    void *allocations[MAX_PROGRAM_ALLOCATIONS];
    PROCESS_FILETYPE filetype;
    union 
    {
       void *ptr;     // physical address of the process
        struct elf_file *elf_file;
    };
    
    void *stack;   // physical address of the stack
    uint32_t size; // size of the data pointed to by "ptr"
};

int process_load(const char *filename, struct process **process);
void *task_get_stack_item(struct task *task, int item);
int task_page_task(struct task *task);
struct process *process_current();
struct process *process_get(int process_id);

int process_switch(struct process *process);
int process_load_switch(const char *filename, struct process **process);
void* process_malloc(struct process* process, size_t size);
void process_free(struct process* process, void* ptr);

#endif