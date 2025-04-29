#ifndef KERNEL_H
#define KERNEL_H

#include <os/types.h>

void kernel_panic(const char *msg) __attribute__((noreturn));

void boot_loadinfo();
void kernel_init();
void kernel_init2();

void kernel_page();
void kernel_registers();
void halt();
void reboot();
void shutdown();
void sync();
u64 get_ticks();

#endif // KERNEL_H