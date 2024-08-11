#ifndef KERNEL_H
#define KERNEL_H

#include <os/types.h>

void kernel_main();

void kernel_panic(const char *msg);

void kernel_page();
void kernel_registers();
void halt();
void reboot();
u64 get_ticks();

#endif // KERNEL_H