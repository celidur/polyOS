#include <os/int80/misc.h>
#include <os/task.h>

void *int80h_command0_sum(struct interrupt_frame *frame)
{
    int v2 = (int)task_get_stack_item(task_current(), 1);
    int v1 = (int)task_get_stack_item(task_current(), 0);
    return (void *)(v1 + v2);
}