#include <os/elfloader.h>
#include <os/file.h>
#include <os/status.h>
#include <os/types.h>
#include <os/memory.h>
#include <os/kheap.h>
#include <os/string.h>
#include <os/paging.h>
#include <os/config.h>

const char elf_signature[] = { 0x7f, 'E', 'L', 'F' };

static bool elf_valid_signature(void* buffer){
    return memcmp(buffer, (void *) elf_signature, sizeof(elf_signature)) == 0;
}

static bool elf_valid_class(struct elf_header* header){
    // we only support 32-bit elf files
    return header->e_ident[EI_CLASS] == ELFCLASSNONE || header->e_ident[EI_CLASS] == ELFCLASS32;
}

static bool elf_valid_encoding(struct elf_header* header){
    return header->e_ident[EI_DATA] == ELFDATANONE || header->e_ident[EI_DATA] == ELFDATA2LSB;
}

static bool elf_is_executable(struct elf_header* header){
    return header->e_type == ET_EXEC && header->e_entry >= PROGRAM_VIRTUAL_ADDRESS;
}

static bool elf_has_program_header(struct elf_header* header){
    return header->e_phoff != 0;
}

void* elf_memory(struct elf_file* file){
    return file->elf_memory;
}

struct elf_header* elf_header(struct elf_file* file){
    return (struct elf_header*) file->elf_memory;
}

struct elf32_shdr* elf_sheader(struct elf_header* header){
    return (struct elf32_shdr*) ((uint32_t) header + header->e_shoff);
}

struct elf32_phdr* elf_pheader(struct elf_header* header){
    if (header->e_phoff == 0){
        return NULL;
    }
    return (struct elf32_phdr*) ((uint32_t) header + header->e_phoff);
}

struct elf32_phdr* elf_program_header(struct elf_header* header, int index){
    return &elf_pheader(header)[index];
}

struct elf32_shdr* elf_section(struct elf_header* header, int index){
    return &elf_sheader(header)[index];
}

char* elf_str_table(struct elf_header* header){
    return (char*) header + elf_section(header, header->e_shstrndx)->sh_offset;
}

void* elf_virtual_base(struct elf_file* file){
    return file->virtual_base_address;
}

void* elf_virtual_end(struct elf_file* file){
    return file->virtual_end_address;
}

void* elf_phys_base(struct elf_file* file){
    return file->physical_base_address;
}

void* elf_phys_end(struct elf_file* file){
    return file->physical_end_address;
}

int elf_validate_loaded(struct elf_header* header){
    return (elf_valid_signature(header) && elf_valid_class(header) && elf_valid_encoding(header) && elf_has_program_header(header) && elf_is_executable(header)) ? ALL_OK : -EINFORMAT;
}

int elf_process_phdr_pt_load(struct elf_file* elf_file, struct elf32_phdr* phdr){
    if (elf_file->virtual_base_address >= (void*) phdr->p_vaddr || elf_file->virtual_base_address == NULL){
        elf_file->virtual_base_address = (void*) phdr->p_vaddr;
        elf_file->physical_base_address = elf_memory(elf_file) + phdr->p_offset;
    }

    unsigned int end_virtual_address = phdr->p_vaddr + phdr->p_filesz;

    if (elf_file->virtual_end_address <= (void*) end_virtual_address || elf_file->virtual_end_address == NULL){
        elf_file->virtual_end_address = (void*) end_virtual_address;
        elf_file->physical_end_address = elf_memory(elf_file) + phdr->p_offset + phdr->p_filesz;
    }

    return ALL_OK;
}

int elf_process_pheader(struct elf_file* elf_file, struct elf32_phdr* phdr){
    switch (phdr->p_type)
    {
    case PT_LOAD:
        return elf_process_phdr_pt_load(elf_file, phdr);
    }

    return ALL_OK;
}

int elf_process_pheaders(struct elf_file* elf_file){
    struct elf_header* header = elf_header(elf_file);

    for (int i = 0; i < header->e_phnum; i++){
        struct elf32_phdr* phdr = elf_program_header(header, i);
        int result = elf_process_pheader(elf_file, phdr);
        if (result != ALL_OK){
            return result;
        }
    }

    return ALL_OK;
}

int elf_process_loaded(struct elf_file* elf_file){
    struct elf_header* header = elf_header(elf_file);

    int result = elf_validate_loaded(header);
    if (result != ALL_OK){
        return result;
    }

    result = elf_process_pheaders(elf_file);
    if (result != ALL_OK){
        return result;
    }

    return ALL_OK;
}

void elf_file_free(struct elf_file* elf_file)
{
    if (elf_file->elf_memory)
    {
        kfree(elf_file->elf_memory);
    }
    kfree(elf_file);
}
struct elf_file* elf_file_new()
{
    return (struct elf_file*)kzalloc(sizeof(struct elf_file));
}

int elf_load(const char* filename, struct elf_file** file_out){
    struct elf_file* elf_file = elf_file_new();
    int fd = 0;
    int res = fopen(filename, "r");
    if (res <= 0){
        return -EIO;
    }

    fd = res;
    struct file_stat stat;
    res = fstat(fd, &stat);
    if (res < 0){
        goto out;
    }

    elf_file->elf_memory = kpalloc(stat.size);
    res = fread(fd, elf_file->elf_memory, stat.size);
    if (res != stat.size){
        goto out;
    }

    res = elf_process_loaded(elf_file);
    if (res < 0){
        goto out;
    }

    *file_out = (struct elf_file*) elf_file;
    
out:
    if (res < 0)
    {
        elf_file_free(elf_file);
    }
    fclose(fd);
    return res;
}

void elf_close(struct elf_file* elf_file){
    if (elf_file == NULL){
        return;
    }

    kfree(elf_file->elf_memory);
    kfree(elf_file);
}

void* elf_phdr_phys_address(struct elf_file* elf_file, struct elf32_phdr* phdr){
    return elf_memory(elf_file) + phdr->p_offset;
}