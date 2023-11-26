#include "kernel.h"
#include <stddef.h>
#include <stdint.h>
#include "idt/idt.h"
#include "memory/heap/kheap.h"
#include "memory/paging/paging.h"

static struct paging_4gb_chunk *kernel_chunk = 0;

uint16_t *vga_buffer = 0;
uint16_t terminal_row = 0;
uint16_t terminal_col = 0;

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
    LIGHT_BROWN = 14,
    WHITE = 15
};

uint16_t terminal_make_char(char c, char color)
{
    return (color << 8) | c;
}

void terminal_put_char(char c, char color, int x, int y)
{
    vga_buffer[x + y * VGA_WIDTH] = terminal_make_char(c, color);
}

void terminal_writechar(char c, char color)
{
    if (c == '\n')
    {
        terminal_col = 0;
        terminal_row++;
        return;
    }
    terminal_put_char(c, color, terminal_col, terminal_row);
    terminal_col++;
    if (terminal_col >= VGA_WIDTH)
    {
        terminal_col = 0;
        terminal_row++;
    }

    if (terminal_row >= VGA_HEIGHT)
    {
        terminal_row = 0;
    }
}

void terminal_initialize()
{
    vga_buffer = (uint16_t *)0xB8000;
    terminal_row = 0;
    terminal_col = 0;
    for (int y = 0; y < VGA_HEIGHT; y++)
    {
        for (int x = 0; x < VGA_WIDTH; x++)
        {
            terminal_put_char(' ', 0, x, y);
        }
    }
}

size_t strlen(const char *str)
{
    size_t len = 0;
    while (str[len])
    {
        len++;
    }
    return len;
}

void print(const char *str)
{
    size_t len = strlen(str);
    for (size_t i = 0; i < len; i++)
    {
        terminal_writechar(str[i], WHITE);
    }
}

void kernel_panic(const char *msg)
{
    size_t len = strlen(msg);
    for (size_t i = 0; i < len; i++)
    {
        terminal_writechar(msg[i], RED);
    }
    while (1)
        ;
}

void kernel_main()
{
    terminal_initialize();
    print("Hello, World!!\n");

    // Initialize kernel heap
    kheap_init();

    // Initialize IDT
    idt_init();

    enable_interrupts();

    // Initialize paging
    kernel_chunk = paging_new_4gb(PAGING_IS_WRITABLE | PAGING_IS_PRESENT | PAGING_ACCESS_FROM_ALL);
    paging_switch(paging_4gb_chunk_get_directory(kernel_chunk));

    char *ptr = kzalloc(4096);
    paging_set(paging_4gb_chunk_get_directory(kernel_chunk), (void *)0x1000, (uint32_t)ptr | PAGING_IS_WRITABLE | PAGING_IS_PRESENT | PAGING_ACCESS_FROM_ALL);

    enable_paging();

    char *ptr2 = (char *)0x1000;
    ptr2[0] = 'a';
    ptr2[1] = 'b';
    print(ptr2);
    print("\n");
    print(ptr);
}