#include <os/paging.h>
#include <os/kheap.h>
#include <os/status.h>
#include <os/kernel.h>
#include <os/terminal.h>

// asm function in src/memory/paging/paging.asm
void paging_load_directory(u32 *directory);

static u32 *current_directory = 0;

static int32_t paging_get_highest_flag(u32* entry){
    u32 flags = 0;
    u32* e = (u32*) ((u32)entry & 0xFFFFF000);
    for(int i = 0; i < PAGING_PAGE_TABLE_SIZE; i++){
        flags |= e[i] & 7;
    }
    return flags;
}
page_t *paging_new_4gb(u8 flags)
{
    u32 *directory = kpalloc(sizeof(u32) * PAGING_PAGE_TABLE_SIZE);
    if (!directory)
    {
        kernel_panic("Failed to allocate page directory");
    }
    u32 *entries = kpalloc(sizeof(u32) * PAGING_PAGE_TABLE_SIZE * PAGING_PAGE_TABLE_SIZE);
    if (!entries)
    {
        kernel_panic("Failed to allocate page table entries");
    }
    u32 offset = 0;
    for (int i = 0; i < PAGING_PAGE_TABLE_SIZE; i++)
    {
        u32 *entry = &entries[i * PAGING_PAGE_TABLE_SIZE];
        if (!entry)
        {
            kernel_panic("Failed to allocate page table");
        }
        for (int b = 0; b < PAGING_PAGE_TABLE_SIZE; b++)
        {
            entry[b] = (offset + (b * PAGING_PAGE_SIZE)) | flags;
        }
        offset += (PAGING_PAGE_TABLE_SIZE * PAGING_PAGE_SIZE);
        directory[i] = ((u32)entry) | flags;
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
    return ((u32)addr % PAGING_PAGE_SIZE) == 0;
}

static int paging_get_index(void *virtual_addr, u32 *directory_index_out, u32 *table_index_out)
{
    if (!paging_is_aligned(virtual_addr))
    {
        return -EINVARG;
    }

    *directory_index_out = ((u32)virtual_addr / (PAGING_PAGE_SIZE * PAGING_PAGE_TABLE_SIZE));
    *table_index_out = ((u32)virtual_addr % (PAGING_PAGE_SIZE * PAGING_PAGE_TABLE_SIZE) / PAGING_PAGE_SIZE);

    return 0;
}

int paging_set(u32 *directory, void *virtual_addr, u32 value)
{
    if (!paging_is_aligned(virtual_addr))
    {
        return -EINVARG;
    }

    u32 directory_index = 0;
    u32 table_index = 0;
    int res = paging_get_index(virtual_addr, &directory_index, &table_index);
    if (res < 0)
    {
        return res;
    }

    u32 entry = directory[directory_index];
    u32 *table = (u32 *)(entry & 0xFFFFF000);
    table[table_index] = value;

    u32 flags = paging_get_highest_flag(table);
    directory[directory_index] = ((u32)table) | flags;

    return 0;
}

void paging_free_4gb(page_t *chunk)
{
    u32 entry = chunk[0];
    u32 *table = (u32 *)(entry & 0xFFFFF000);
    kfree(table);
    kfree(chunk);
}

void *paging_align_address(void *addr)
{
    if (!paging_is_aligned(addr))
    {
        return (void *)((u32)addr & 0xFFFFF000) + PAGING_PAGE_SIZE;
    }
    return addr;
}

int paging_map(page_t *directory, void *virt, void *phys, u8 flags)
{
    if (!paging_is_aligned(virt) || !paging_is_aligned(phys))
        return -EINVARG;
    return paging_set(directory, virt, (u32)phys | flags);
}

int paging_map_range(page_t *directory, void *virt, void *phys, int count, u8 flags)
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

int paging_map_to(page_t *directory, void *virt, void *phys, void *phys_end, u8 flags)
{
    if (!paging_is_aligned(virt))
        return -EINVARG;

    if (!paging_is_aligned(phys))
        return -EINVARG;

    if (!paging_is_aligned(phys_end))
        return -EINVARG;

    if ((u32)phys_end < (u32)phys)
        return -EINVARG;

    u32 total_bytes = phys_end - phys;
    u32 total_pages = total_bytes / PAGING_PAGE_SIZE;
    return paging_map_range(directory, virt, phys, total_pages, flags);
}

u32 paging_get(u32 *directory, void *virtual_addr)
{
    if (!paging_is_aligned(virtual_addr))
    {
        return -EINVARG;
    }

    u32 directory_index = 0;
    u32 table_index = 0;
    paging_get_index(virtual_addr, &directory_index, &table_index);
    u32 entry = directory[directory_index];
    u32 *table = (u32 *)(entry & 0xFFFFF000);
    return table[table_index];
}

void* paging_align_to_lower_page(void* addr){
    return (void*) ((u32) addr & 0xFFFFF000);
}

void* paging_get_physical_address(u32* directory, void* virtual_address){
    void* virt_addr_new = (void*) paging_align_to_lower_page(virtual_address);
    void* difference = (void*) ((u32) virtual_address - (u32) virt_addr_new);
    return (void*) ((paging_get(directory, virt_addr_new) & 0xFFFFF000) + (u32) difference);
}

void print_paging_info(u32* directory){
    serial_printf("Paging info: \n");
    u32 flag = 0;
    u32 start = 0xFFFFFFFF;
    u32 end = 0;
    for(u32 i = 0; i < PAGING_PAGE_TABLE_SIZE; i++){
        u32* entry = (u32*) ((u32)directory[i] & 0xFFFFF000);
        for(u32 b = 0; b < PAGING_PAGE_TABLE_SIZE; b++){
            u32 flag2 = entry[b] & 31;
            if (flag2 != flag){
                if (start != 0xFFFFFFFF){
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
            end = (i * PAGING_PAGE_SIZE * PAGING_PAGE_TABLE_SIZE) + (b * PAGING_PAGE_SIZE) + 0xFFF;
        }
    }
    if (start != 0xFFFFFFFF){
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