#include "kernel.h"
#include <stddef.h>
#include <stdint.h>
#include "idt/idt.h"

uint16_t* vga_buffer = 0;
uint16_t terminal_row = 0;
uint16_t terminal_col = 0;

uint16_t terminal_make_char(char c, char color) {
    return (color << 8) | c;
}

void terminal_put_char(char c, char color, int x, int y) {
    vga_buffer[x + y * VGA_WIDTH] = terminal_make_char(c, color);
}

void terminal_writechar(char c, char color) {
    if (c == '\n') {
        terminal_col = 0;
        terminal_row++;
        return;
    } 
    terminal_put_char(c, color, terminal_col, terminal_row);
    terminal_col++;
    if (terminal_col >= VGA_WIDTH) {
        terminal_col = 0;
        terminal_row++;
    }

    if (terminal_row >= VGA_HEIGHT) {
        terminal_row = 0;
    }
    
}

void terminal_initialize() {
    vga_buffer = (uint16_t*) 0xB8000;
    terminal_row = 0;
    terminal_col = 0;
    for (int y = 0; y < VGA_HEIGHT; y++) {
        for (int x = 0; x < VGA_WIDTH; x++) {
            terminal_put_char(' ', 0, x, y);
        }
    }
}

size_t strlen(const char* str) {
    size_t len = 0;
    while (str[len]) {
        len++;
    }
    return len;
}

void print(const char* str) {
    size_t len = strlen(str);
    for (size_t i = 0; i < len; i++) {
        terminal_writechar(str[i], 15);
    }
}

void kernel_main() {
    terminal_initialize();
    print("Hello, World!!\n");

    idt_init();

}