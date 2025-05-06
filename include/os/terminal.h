#ifndef TERMINAL_H
#define TERMINAL_H

#include <os/types.h>

enum color
{
    BLACK = 0,
    BLUE = 1,
    GREEN = 2,
    CYAN = 3,
    RED = 4,
    MAGENTA = 5,
    BROWN = 6,
    LIGHT_GREY = 7,
    DARK_GREY = 8,
    LIGHT_BLUE = 9,
    LIGHT_GREEN = 10,
    LIGHT_CYAN = 11,
    LIGHT_RED = 12,
    LIGHT_MAGENTA = 13,
    YELLOW = 14,
    WHITE = 15
};

typedef u8 color_t;

void set_color(color_t background, color_t foreground);

void print_c(const char *str, color_t color);
void print(const char *str);
void terminal_writechar(u8 c, color_t color);
int printf(const char *fmt, ...);
int serial_printf(const char *fmt, ...);
void clear_screen();
void disable_cursor();

#endif