#ifndef GDT_H
#define GDT_H

#include <os/types.h>

struct gdt
{
    u16 segment;
    u16 base_first;
    u8 base;
    u8 access;
    u8 high_flags;
    u8 base_24_31_bits;
} __attribute__((packed));

struct gdt_struct
{
    u32 base;
    u32 limit;
    u8 type;
};

void gdt_load(struct gdt *gdt, int size);
void gdt_struct_to_gdt(struct gdt_struct *gdt_struct, struct gdt *gdt, int total_entries);

#endif