#include "io.h"
#include "task/task.h"
#include "kernel.h"
#include "keyboard/keyboard.h"

void *int80h_command1_print(struct interrupt_frame *frame)
{
    void *user_memoire = task_get_stack_item(task_current(), 0);
    char buff[1024];
    copy_string_from_task(task_current(), user_memoire, buff, 1024);
    print(buff);
    return 0;
}

void *int80h_command2_getkey(struct interrupt_frame *frame)
{
    char c = keyboard_pop();
    return (void *)(uint32_t)c;
}

void *int80h_command3_putchar(struct interrupt_frame *frame)
{
    char c = (char)(int) task_get_stack_item(task_current(), 0);
    terminal_writechar(c, 15);
    return 0;
}