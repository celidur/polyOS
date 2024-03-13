#ifndef KEYBOARD_H
#define KEYBOARD_H

#include <os/types.h>

#define ESC (0x1B)
#define BS  (0x08)
#define ENTER (0x0D)

struct process;
typedef int (*KEYBOARD_INIT_FUNC)();

struct keyboard
{
    KEYBOARD_INIT_FUNC init;
    char name[20];
    u8 shift;
    u8 ctrl;
    struct keyboard *next;
};

void keyboard_init();
void keyboard_backspace(struct process *proc);
void keyboard_push(char c);
char keyboard_pop();
int keyboard_insert(struct keyboard *keyboard);

#endif