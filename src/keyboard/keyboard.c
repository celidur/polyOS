#include <os/keyboard.h>
#include <os/status.h>
#include <os/process.h>
#include <os/task.h>
#include <os/classic.h>

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