#ifndef VGA_H
#define VGA_H

#include <os/types.h>

enum screen_mode {
    VGA_40x25_TEXT,
    VGA_40x50_TEXT,
    VGA_80x25_TEXT,
    VGA_80x50_TEXT,
    VGA_90x30_TEXT,
    VGA_90x60_TEXT,
    VGA_640x480x2,
    VGA_320x200x4,
    VGA_640x480x16,
    VGA_720x480x16,
    VGA_320x200x256,
    VGA_320x200x256_MODEX,
};

void dump_state(void);
void set_text_mode(enum screen_mode mode);
void set_graphics_mode(enum screen_mode mode);
void set_pixel(u32 x, u32 y, u32 color);

int get_screen_width(void);
int get_screen_height(void);


#endif
