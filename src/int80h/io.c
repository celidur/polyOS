#include "io.h"
#include "task/task.h"
#include "kernel.h"

void *int80h_command1_print(struct interrupt_frame *frame)
{
    void *user_memoire = task_get_stack_item(task_current(), 0);
    char buff[1024];
    copy_string_from_task(task_current(), user_memoire, buff, 1024);
    print(buff);
    return 0;
}