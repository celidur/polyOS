#include "kernel.h"
#include <stddef.h>
#include <stdint.h>
#include "idt/idt.h"
#include "memory/heap/kheap.h"
#include "memory/paging/paging.h"
#include "disk/disk.h"
#include "string/string.h"
#include "fs/pparser.h"
#include "disk/streamer.h"
#include "fs/file.h"
#include "memory/memory.h"
#include "gdt/gdt.h"
#include "config.h"
#include "task/tss.h"
#include "task/task.h"
#include "task/process.h"
#include "status.h"
#include "int80h/int80.h"

struct tss tss;
static struct paging_4gb_chunk *kernel_chunk = 0;
struct gdt gdt_real[TOTAL_GDT_SEGMENTS];
struct gdt_struct gdt_struct[TOTAL_GDT_SEGMENTS] = {
    {.base = 0x00, .limit = 0x00, .type = 0x00},                  // NULL Segment
    {.base = 0x00, .limit = 0xFFFFFFFF, .type = 0x9A},            // Kernel code
    {.base = 0x00, .limit = 0xFFFFFFFF, .type = 0x92},            // Kernel data
    {.base = 0x00, .limit = 0xFFFFFFFF, .type = 0xF8},            // User code
    {.base = 0x00, .limit = 0xFFFFFFFF, .type = 0xF2},            // User data
    {.base = (uint32_t)&tss, .limit = sizeof(tss), .type = 0xE9}, // TSS segment

};

uint16_t *vga_buffer = 0;
uint16_t terminal_row = 0;
uint16_t terminal_col = 0;

void kernel_page()
{
    kernel_registers();
    paging_switch(kernel_chunk);
}

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

char *itoa(int num, char *buffer)
{
    int current = 0;
    if (num == 0)
    {
        buffer[current++] = '0';
        buffer[current] = '\0';
        return buffer;
    }
    int is_negative = 0;
    if (num < 0)
    {
        is_negative = 1;
        num = -num;
    }
    while (num != 0)
    {
        int digit = num % 10;
        buffer[current++] = digit + '0';
        num /= 10;
    }
    if (is_negative)
    {
        buffer[current++] = '-';
    }
    buffer[current] = '\0';
    int len = strlen(buffer);
    for (int i = 0; i < len / 2; i++)
    {
        char tmp = buffer[i];
        buffer[i] = buffer[len - i - 1];
        buffer[len - i - 1] = tmp;
    }
    return buffer;
}

void print_int(int value)
{
    char buffer[32];
    itoa(value, buffer);
    print(buffer);
    print("\n");
}

void kernel_main()
{
    terminal_initialize();
    print("Hello, World!!\n");

    memset(gdt_real, 0, sizeof(gdt_real));
    gdt_struct_to_gdt(gdt_struct, gdt_real, TOTAL_GDT_SEGMENTS);

    // Load GDT
    gdt_load(gdt_real, sizeof(gdt_real));

    // Initialize kernel heap
    kheap_init();

    // Initialize filesystems
    fs_init();

    // Initialize disks
    disk_search_and_init();

    // Initialize IDT
    idt_init();
    enable_interrupts();

    // Initialize TSS
    memset(&tss, 0, sizeof(tss));
    tss.esp0 = 0x600000;
    tss.ss0 = KERNEL_DATA_SELECTOR;

    tss_load(0x28);

    // Initialize paging
    kernel_chunk = paging_new_4gb(PAGING_IS_WRITABLE | PAGING_IS_PRESENT | PAGING_ACCESS_FROM_ALL);
    paging_switch(kernel_chunk);
    enable_paging();

    // initialize interrupts 80h
    int80h_register_commands();

    print("Kernel initialized\n");

    struct process *process = NULL;
    int res = process_load("0:/blank.bin", &process);
    if (res < 0)
    {
        kernel_panic("Failed to load process\n");
    }

    task_run_first_ever_task();

    // Never reached
}