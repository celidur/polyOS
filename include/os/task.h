#ifndef TASK_H
#define TASK_H

typedef unsigned int		u32;

struct registers
{
    u32 edi;
    u32 esi;
    u32 ebp;
    u32 ebx;
    u32 edx;
    u32 ecx;
    u32 eax;

    u32 ip;
    u32 cs;
    u32 flags;
    u32 esp;
    u32 ss;
}__attribute__((packed));



void task_return(struct registers *regs) __attribute__((noreturn));
void user_registers();

#endif
