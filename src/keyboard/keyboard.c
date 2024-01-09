#include "keyboard.h"
#include "status.h"
#include "kernel.h"
#include "task/process.h"
#include "task/task.h"
#include "classic.h"

static struct keyboard *keyboard_list_head = NULL;
static struct keyboard *keyboard_list_tail = NULL;

void keyboard_init()
{
    keyboard_insert(classic_init());
}

int keyboard_insert(struct keyboard *keyboard)
{
    if (keyboard->init == NULL)
    {
        return -EINVARG;
    }
    if (keyboard_list_tail)
    {
        keyboard_list_tail->next = keyboard;
        keyboard_list_tail = keyboard;
    }
    else
    {
        keyboard_list_head = keyboard;
        keyboard_list_tail = keyboard;
    }

    return keyboard->init();
}

static int keyboard_get_tail_index(struct process *process)
{
    return (process->keyboard.tail) % sizeof(process->keyboard.buffer);
}

void keyboard_backspace(struct process *process)
{
    process->keyboard.tail--;
    int index = keyboard_get_tail_index(process);
    process->keyboard.buffer[index] = '\0';
}

void keyboard_push(char c)
{
    struct process *process = process_current();
    if (!process)
    {
        return;
    }
    int index = keyboard_get_tail_index(process);
    process->keyboard.buffer[index] = c;
    process->keyboard.tail++;
}

char keyboard_pop()
{
    struct process *process = process_current();
    if (!process)
    {
        return 0;
    }
    int index = process->keyboard.head % sizeof(process->keyboard.buffer);
    char c = process->keyboard.buffer[index];
    if (c == 0x00)
    {
        return 0;
    }
    process->keyboard.buffer[index] = 0x00;
    process->keyboard.head++;
    return c;
}