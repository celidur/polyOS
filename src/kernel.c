#include "kernel.h"
#include "idt/idt.h"
#include "memory/heap/kheap.h"
#include "memory/paging/paging.h"
#include "disk/disk.h"
#include "memory/memory.h"
#include "gdt/gdt.h"
#include "task/tss.h"
#include "task/process.h"
#include "int80h/int80.h"
#include "keyboard/keyboard.h"
#include "terminal/terminal.h"
#include "terminal/serial.h"

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
    printf(msg);
    while (1)
        ;
}

void kernel_main()
{
    terminal_initialize();
    serial_configure(SERIAL_COM1_BASE, Baud_115200);

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
    
    struct process *process = NULL;
    int res = process_load_switch("0:/shell.elf", &process);
    if (res < 0)
    {
        kernel_panic("Failed to load process\n");
    }

    set_color(BLACK, LIGHT_GREEN);
    print_memory();

    set_color(BLACK, WHITE);

    print_paging_info(process->task->page_directory);

    // int res = process_load_switch("0:/blank.elf", &process);
    // if (res < 0)
    // {
    //     kernel_panic("Failed to load process\n");
    // }

    // struct command_argument arg;
    // strcpy(arg.argument, "TEST");
    // arg.next = NULL;
    // process_inject_arguments(process, &arg);

    // res = process_load_switch("0:/blank.elf", &process);
    // if (res < 0)
    // {
    //     kernel_panic("Failed to load process\n");
    // }
    // strcpy(arg.argument, "TEST2");
    // process_inject_arguments(process, &arg);

    task_run_first_ever_task();

    // Never reached
}