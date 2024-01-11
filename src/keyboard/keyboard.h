#ifndef KEYBOARD_H
#define KEYBOARD_H

#define KEYBOARD_CAPS_LOCK_ON 0x01
#define KEYBOARD_CAPS_LOCK_OFF 0x00

struct process;
typedef int KEYBOARD_CAPS_LOCK_STATE;
typedef int (*KEYBOARD_INIT_FUNC)();

struct keyboard
{
    KEYBOARD_INIT_FUNC init;
    char name[20];
    KEYBOARD_CAPS_LOCK_STATE caps_lock_state;
    struct keyboard *next;
};

void keyboard_init();
void keyboard_backspace(struct process *proc);
void keyboard_push(char c);
char keyboard_pop();
int keyboard_insert(struct keyboard *keyboard);
void keyboard_set_capslock(struct keyboard *keyboard, KEYBOARD_CAPS_LOCK_STATE state);
KEYBOARD_CAPS_LOCK_STATE keyboard_get_capslock(struct keyboard *keyboard);

#endif