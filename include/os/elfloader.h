#ifndef ELF_LOADER_H
#define ELF_LOADER_H

#include <os/elf.h>
#include <os/config.h>

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

void elf_close(struct elf_file* elf_file);
int elf_load(const char* filename, struct elf_file** file_out);
struct elf_file* elf_file_new();
void elf_file_free(struct elf_file* file);

void* elf_virtual_base(struct elf_file* file);
void* elf_virtual_end(struct elf_file* file);
void* elf_phys_base(struct elf_file* file);
void* elf_phys_end(struct elf_file* file);

struct elf32_shdr* elf_sheader(struct elf_header* header);
struct elf_header* elf_header(struct elf_file* file);
void* elf_memory(struct elf_file* file);
struct elf32_phdr* elf_pheader(struct elf_header* header);
struct elf32_phdr* elf_program_header(struct elf_header* header, int index);
struct elf32_shdr* elf_section(struct elf_header* header, int index);
void* elf_phdr_phys_address(struct elf_file* elf_file, struct elf32_phdr* phdr);

#endif