#ifndef PROCESS_H
#define PROCESS_H
#include <stdint.h>
#include "task.h"
#include "config.h"

struct process
{
    uint16_t pid;
    char filename[MAX_PATH];
    struct task *task;

    void *allocations[MAX_PROGRAM_ALLOCATIONS];
    void *ptr;     // physical address of the process
    void *stack;   // physical address of the stack
    uint32_t size; // size of the data pointed to by "ptr"
};

int process_load(const char *filename, struct process **process);

#endif