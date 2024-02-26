#include "paging.h"
#include "memory/heap/kheap.h"
#include "status.h"
#include "kernel.h"
#include "terminal/terminal.h"
#include "terminal/serial.h"

// asm function in src/memory/paging/paging.asm
void paging_load_directory(uint32_t *directory);

static uint32_t *current_directory = 0;

static int32_t paging_get_highest_flag(uint32_t* entry){
    uint32_t flags = 0;
    uint32_t* e = (uint32_t*) ((uint32_t)entry & 0xFFFFF000);
    for(int i = 0; i < PAGING_PAGE_TABLE_SIZE; i++){
        flags |= e[i] & 7;
    }
    return flags;
}
page_t *paging_new_4gb(uint8_t flags)
{
    uint32_t *directory = kzalloc(sizeof(uint32_t) * PAGING_PAGE_TABLE_SIZE);
    if (!directory)
    {
        kernel_panic("Failed to allocate page directory");
    }
    uint32_t offset = 0;
    for (int i = 0; i < PAGING_PAGE_TABLE_SIZE; i++)
    {
        uint32_t *entry = kzalloc(sizeof(uint32_t) * PAGING_PAGE_TABLE_SIZE);
        if (!entry)
        {
            kernel_panic("Failed to allocate page table");
        }
        for (int b = 0; b < PAGING_PAGE_TABLE_SIZE; b++)
        {
            entry[b] = (offset + (b * PAGING_PAGE_SIZE)) | flags;
        }
        offset += (PAGING_PAGE_TABLE_SIZE * PAGING_PAGE_SIZE);
        directory[i] = ((uint32_t)entry) | flags;
    }

    return directory;
}

void paging_switch(page_t *directory)
{
    paging_load_directory(directory);
    current_directory = directory;
}

bool paging_is_aligned(void *addr)
{
    return ((uint32_t)addr % PAGING_PAGE_SIZE) == 0;
}

static int paging_get_index(void *virtual_addr, uint32_t *directory_index_out, uint32_t *table_index_out)
{
    if (!paging_is_aligned(virtual_addr))
    {
        return -EINVARG;
    }

    *directory_index_out = ((uint32_t)virtual_addr / (PAGING_PAGE_SIZE * PAGING_PAGE_TABLE_SIZE));
    *table_index_out = ((uint32_t)virtual_addr % (PAGING_PAGE_SIZE * PAGING_PAGE_TABLE_SIZE) / PAGING_PAGE_SIZE);

    return 0;
}

int paging_set(uint32_t *directory, void *virtual_addr, uint32_t value)
{
    if (!paging_is_aligned(virtual_addr))
    {
        return -EINVARG;
    }

    uint32_t directory_index = 0;
    uint32_t table_index = 0;
    int res = paging_get_index(virtual_addr, &directory_index, &table_index);
    if (res < 0)
    {
        return res;
    }

    uint32_t entry = directory[directory_index];
    uint32_t *table = (uint32_t *)(entry & 0xFFFFF000);
    table[table_index] = value;

    uint32_t flags = paging_get_highest_flag(table);
    directory[directory_index] = ((uint32_t)table) | flags;

    return 0;
}

void paging_free_4gb(page_t *chunk)
{
    for (int i = 0; i < PAGING_PAGE_TABLE_SIZE; i++)
    {
        uint32_t entry = chunk[i];
        uint32_t *table = (uint32_t *)(entry & 0xFFFFF000);
        kfree(table);
    }
    kfree(chunk);
}

void *paging_align_address(void *addr)
{
    if (!paging_is_aligned(addr))
    {
        return (void *)((uint32_t)addr & 0xFFFFF000) + PAGING_PAGE_SIZE;
    }
    return addr;
}

int paging_map(page_t *directory, void *virt, void *phys, uint8_t flags)
{
    if (!paging_is_aligned(virt) || !paging_is_aligned(phys))
        return -EINVARG;
    return paging_set(directory, virt, (uint32_t)phys | flags);
}

int paging_map_range(page_t *directory, void *virt, void *phys, int count, uint8_t flags)
{
    for (int i = 0; i < count; i++)
    {
        if (paging_map(directory, virt, phys, flags) < 0)
            return -EIO;
        virt += PAGING_PAGE_SIZE;
        phys += PAGING_PAGE_SIZE;
    }
    return ALL_OK;
}

int paging_map_to(page_t *directory, void *virt, void *phys, void *phys_end, uint8_t flags)
{
    if (!paging_is_aligned(virt))
        return -EINVARG;

    if (!paging_is_aligned(phys))
        return -EINVARG;

    if (!paging_is_aligned(phys_end))
        return -EINVARG;

    if ((uint32_t)phys_end < (uint32_t)phys)
        return -EINVARG;

    uint32_t total_bytes = phys_end - phys;
    uint32_t total_pages = total_bytes / PAGING_PAGE_SIZE;
    return paging_map_range(directory, virt, phys, total_pages, flags);
}

uint32_t paging_get(uint32_t *directory, void *virtual_addr)
{
    if (!paging_is_aligned(virtual_addr))
    {
        return -EINVARG;
    }

    uint32_t directory_index = 0;
    uint32_t table_index = 0;
    paging_get_index(virtual_addr, &directory_index, &table_index);
    uint32_t entry = directory[directory_index];
    uint32_t *table = (uint32_t *)(entry & 0xFFFFF000);
    return table[table_index];
}

void* paging_align_to_lower_page(void* addr){
    return (void*) ((uint32_t) addr & 0xFFFFF000);
}

void* paging_get_physical_address(uint32_t* directory, void* virtual_address){
    void* virt_addr_new = (void*) paging_align_to_lower_page(virtual_address);
    void* difference = (void*) ((uint32_t) virtual_address - (uint32_t) virt_addr_new);
    return (void*) ((paging_get(directory, virt_addr_new) & 0xFFFFF000) + (uint32_t) difference);
}

void print_paging_info(uint32_t* directory){
    serial_printf("Paging info: \n");
    uint32_t flag = 0;
    uint32_t start = -1;
    uint32_t end = -1;
    for(int i = 0; i < PAGING_PAGE_TABLE_SIZE; i++){
        uint32_t* entry = (uint32_t*) ((uint32_t)directory[i] & 0xFFFFF000);
        for(int b = 0; b < PAGING_PAGE_TABLE_SIZE; b++){
            uint32_t flag2 = entry[b] & 31;
            if (flag2 != flag){
                if (start != -1){
                    serial_printf("0x%x - 0x%x: ", start, end);
                    if (flag & PAGING_IS_PRESENT){
                        serial_printf("PRESENT ");
                    }
                    if (flag & PAGING_IS_WRITABLE){
                        serial_printf("WRITABLE ");
                    }
                    if (flag & PAGING_ACCESS_FROM_ALL){
                        serial_printf("ACCESS_FROM_ALL ");
                    }
                    if (flag & PAGING_WRITE_THROUGH){
                        serial_printf("WRITE_THROUGH ");
                    }
                    if (flag & PAGING_CACHE_DISABLED){
                        serial_printf("CACHE_DISABLED ");
                    }
                    serial_printf("\n");
                }
                start = (i * PAGING_PAGE_SIZE * PAGING_PAGE_TABLE_SIZE) + (b * PAGING_PAGE_SIZE);
                flag = flag2;
            } 
            end = (i * PAGING_PAGE_SIZE * PAGING_PAGE_TABLE_SIZE) + (b * PAGING_PAGE_SIZE);
        }
    }
    if (start != -1){
        serial_printf("0x%x - 0x%x: ", start, end);
        if (flag & PAGING_IS_PRESENT){
            serial_printf("PRESENT ");
        }
        if (flag & PAGING_IS_WRITABLE){
            serial_printf("WRITABLE ");
        }
        if (flag & PAGING_ACCESS_FROM_ALL){
            serial_printf("ACCESS_FROM_ALL ");
        }
        if (flag & PAGING_WRITE_THROUGH){
            serial_printf("WRITE_THROUGH ");
        }
        if (flag & PAGING_CACHE_DISABLED){
            serial_printf("CACHE_DISABLED ");
        }
        serial_printf("\n");
    }
}