#include "misc.h"
#include "idt/idt.h"
#include "kernel.h"
#include "task/task.h"

void *int80h_commando_sum(struct interrupt_frame *frame)
{
    int v2 = (int)task_get_stack_item(task_current(), 1);
    int v1 = (int)task_get_stack_item(task_current(), 0);
    print("Sum: ");
    print_int(v1);
    print(" + ");
    print_int(v2);
    print(" = ");
    print_int(v1 + v2);
    print("\n");
    return (void *)(v1 + v2);
}