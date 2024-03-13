#ifndef KERNEL_H
#define KERNEL_H

void kernel_main();

void kernel_panic(const char *msg);

void kernel_page();
void kernel_registers();

#endif // KERNEL_H