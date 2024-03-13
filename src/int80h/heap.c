#include <os/int80/heap.h>
#include <os/kheap.h>
#include <os/task.h>
#include <os/process.h>
#include <os/types.h>

void* int80h_command4_malloc(struct interrupt_frame *frame){
    u32 size = (u32)task_get_stack_item(task_current(), 0);
    return process_malloc(task_current()->process, size);
}

void* int80h_command5_free(struct interrupt_frame *frame){
    void* ptr = (void*)task_get_stack_item(task_current(), 0);
    process_free(task_current()->process, ptr);
    return NULL;
}

void* int80h_command10_print_memory(struct interrupt_frame *frame){
    print_memory();
    return NULL;
}