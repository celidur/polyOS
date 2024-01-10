#include "heap.h"
#include "task/task.h"
#include "task/process.h"
#include <stddef.h>

void* int80h_command4_malloc(struct interrupt_frame *frame){
    size_t size = (size_t)task_get_stack_item(task_current(), 0);
    return process_malloc(task_current()->process, size);
}

void* int80h_command5_free(struct interrupt_frame *frame){
    void* ptr = (void*)task_get_stack_item(task_current(), 0);
    process_free(task_current()->process, ptr);
    return NULL;
}