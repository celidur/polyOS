#include "paging.h"
#include "memory/heap/kheap.h"
#include "status.h"

// asm function in src/memory/paging/paging.asm
void paging_load_directory(uint32_t *directory);

static uint32_t *current_directory = 0;
struct paging_4gb_chunk *paging_new_4gb(uint8_t flags)
{
    uint32_t *directory = kzalloc(sizeof(uint32_t) * PAGING_PAGE_TABLE_SIZE);
    int offset = 0;
    for (int i = 0; i < PAGING_PAGE_TABLE_SIZE; i++)
    {
        uint32_t *entry = kzalloc(sizeof(uint32_t) * PAGING_PAGE_TABLE_SIZE);
        for (int b = 0; b < PAGING_PAGE_TABLE_SIZE; b++)
        {
            entry[b] = (offset + (b * PAGING_PAGE_SIZE)) | flags;
        }
        offset += (PAGING_PAGE_TABLE_SIZE * PAGING_PAGE_SIZE);
        directory[i] = ((uint32_t)entry) | flags | PAGING_IS_WRITABLE;
    }

    struct paging_4gb_chunk *chunk = kzalloc(sizeof(struct paging_4gb_chunk));
    chunk->page_directory = directory;
    return chunk;
}

void paging_switch(uint32_t *directory)
{
    paging_load_directory(directory);
    current_directory = directory;
}

uint32_t *paging_4gb_chunk_get_directory(struct paging_4gb_chunk *chunk)
{
    return chunk->page_directory;
}

bool paging_is_aligned(void *addr)
{
    return ((uint32_t)addr % PAGING_PAGE_SIZE) == 0;
}

int paging_get_index(void *virtual_addr, uint32_t *directory_index_out, uint32_t *table_index_out)
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

    return 0;
}