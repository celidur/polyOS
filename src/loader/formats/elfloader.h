#ifndef ELFLOADER_H
#define ELFLOADER_H

#include <stdint.h>
#include <stddef.h>

#include "elf.h"
#include "config.h"

struct elf_file {
    char filename[MAX_PATH];

    int in_memory_size;

    // The physical address where the elf file is loaded in memory
    void* elf_memory;

    // The virtual base address of this binary
    void* virtual_base_address;

    // the ending virtual address
    void* virtual_end_address;

    // The physical base address of this binary
    void* physical_base_address;

    // The ending physical address
    void* physical_end_address;
};

#endif