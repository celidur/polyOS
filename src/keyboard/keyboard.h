#ifndef KEYBOARD_H
#define KEYBOARD_H

struct process;
typedef int (*KEYBOARD_INIT_FUNC)();

struct keyboard
{
    KEYBOARD_INIT_FUNC init;
    char name[20];
    struct keyboard *next;
};

void keyboard_init();
void keyboard_backspace(struct process *proc);
void keyboard_push(char c);
char keyboard_pop();
int keyboard_insert(struct keyboard *keyboard);

#endif