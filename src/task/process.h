#ifndef PROCESS_H
#define PROCESS_H
#include <stdint.h>
#include "task.h"
#include "config.h"

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
    void *ptr;     // physical address of the process
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

#endif