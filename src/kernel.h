#ifndef KERNEL_H
#define KERNEL_H

#define ERROR(value) (void *)value
#define ERROR_I(value) (int)value
#define ISERR(value) ((int)value < 0)

void kernel_main();

void kernel_panic(const char *msg);

void kernel_page();
void kernel_registers();

#endif // KERNEL_H