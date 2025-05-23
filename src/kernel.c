#include <os/kernel.h>
#include <os/idt.h>
#include <os/kheap.h>
#include <os/paging.h>
#include <os/memory.h>
#include <os/gdt.h>
#include <os/tss.h>
#include <os/process.h>
#include <os/int80/int80.h>
#include <os/keyboard.h>
#include <os/terminal.h>

#include <os/io.h>
#include <os/idt.h>

struct tss tss;
static page_t *kernel_chunk = 0;
struct gdt gdt_real[TOTAL_GDT_SEGMENTS];
struct gdt_struct gdt_struct[TOTAL_GDT_SEGMENTS] = {
    {.base = 0x00, .limit = 0x00, .type = 0x00},                  // NULL Segment
    {.base = 0x00, .limit = 0xFFFFFFFF, .type = 0x9A},            // Kernel code
    {.base = 0x00, .limit = 0xFFFFFFFF, .type = 0x92},            // Kernel data
    {.base = 0x00, .limit = 0xFFFFFFFF, .type = 0xF8},            // User code
    {.base = 0x00, .limit = 0xFFFFFFFF, .type = 0xF2},            // User data
    {.base = (uint32_t)&tss, .limit = sizeof(tss), .type = 0xE9}, // TSS segment

};

void kernel_page()
{
    kernel_registers();
    paging_switch(kernel_chunk);
}

void kernel_panic(const char *msg)
{
    disable_interrupts();
    set_color(BLACK, LIGHT_RED);
    print("\nKERNEL PANIC: ");
    set_color(BLACK, RED);
    print(msg);
    disable_cursor();
    serial_printf("KERNEL PANIC: %s\n", msg);
    halt();
    while (1)
        ;
}

void init_gdt(){
    memset(gdt_real, 0, sizeof(gdt_real));
    gdt_struct_to_gdt(gdt_struct, gdt_real, TOTAL_GDT_SEGMENTS);

    // Load GDT
    gdt_load(gdt_real, sizeof(gdt_real)-1);
}

void kernel_init2()
{
    // Initialize IDT
    idt_init();

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

    // Initialize keyboard
    keyboard_init();
}

u64 get_ticks()
{
    uint32_t low, high;
    asm volatile("rdtsc" : "=a" (low), "=d" (high));
    return low | ((u64)high << 32);
}

void shutdown()
{
    serial_printf("Shutting down...\n");

    outw(0x604, 0x2000);

    halt();
}

void reboot()
{
    uint8_t good = 0x02;
    disable_interrupts();
    while (good & 0x02)
        good = inb(0x64);
    serial_printf("Rebooting...\n");
    outb(0x64, 0xFE);
    halt();
}