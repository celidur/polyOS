#ifndef KERNEL_H
#define KERNEL_H

#define ERROR(value) (void *)value
#define ERROR_I(value) (int)value
#define ISERR(value) ((int)value < 0)

#define VGA_WIDTH 80
#define VGA_HEIGHT 20

void kernel_main();

void print(const char *str);
void terminal_writechar(char c, char color);

void print_int(int value);

void kernel_panic(const char *msg);

void kernel_page();
void kernel_registers();

#endif // KERNEL_H