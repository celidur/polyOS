#ifndef FONT_H
#define FONT_H

#include <os/types.h>


enum vga_font{
    VGA_FONT_8x8,
    VGA_FONT_8x16,
};

uint8_t* get_font(enum vga_font font);

#endif
