#include "heap.h"
#include "kernel.h"
#include "status.h"
#include "memory/memory.h"
#include <stdbool.h>

static int heap_validate(void *ptr, void *end, struct heap_table *table)
{
    int res = 0;

    size_t table_size = (size_t)(end - ptr);
    size_t block_size = table_size / HEAP_SIZE_BLOCKS;

    if (table->total != block_size)
    {
        res = -EINVARG;
        goto out;
    }

out:
    return res;
}

static bool heap_validate_alignment(void *ptr)
{
    return ((unsigned int)ptr % HEAP_SIZE_BLOCKS) == 0;
}

int heap_create(struct heap *heap, void *ptr, void *end, struct heap_table *table)
{
    int res = 0;

    if (!heap_validate_alignment(ptr) || !heap_validate_alignment(end))
    {
        res = -EINVARG;
        goto out;
    }

    memset(heap, 0, sizeof(struct heap));
    heap->saddr = ptr;
    heap->table = table;

    res = heap_validate(ptr, end, table);
    if (res < 0)
    {
        goto out;
    }

    size_t table_size = sizeof(HEAP_BLOCK_TABLE_ENTRY) * table->total;
    memset(table->entries, HEAP_BLOCK_TABLE_ENTRY_FREE, table_size);

out:
    return res;
}

static uint32_t heap_align_value_to_upper(uint32_t val)
{
    if (val % HEAP_SIZE_BLOCKS == 0)
    {
        return val;
    }

    val = (val - (val % HEAP_SIZE_BLOCKS)) + HEAP_SIZE_BLOCKS;
    return val;
}

static int heap_get_entry_type(HEAP_BLOCK_TABLE_ENTRY entry)
{
    return entry & 0x0f;
}

// function to find a contiguous block of memory
static int heap_get_start_block(struct heap *heap, uint32_t total_block)
{
    struct heap_table *table = heap->table;
    int bc = 0;
    int bs = -1;

    for (size_t i = 0; i < table->total; i++)
    {
        if (heap_get_entry_type(table->entries[i]) != HEAP_BLOCK_TABLE_ENTRY_FREE)
        {
            bc = 0;
            bs = -1;
            continue;
        }
        if (bs == -1)
        {
            bs = i;
        }
        bc++;
        if (bc == total_block)
        {
            return bs;
        }
    }
    return -ENOMEM;
}

static void *heap_block_to_address(struct heap *heap, int block)
{
    return heap->saddr + (block * HEAP_SIZE_BLOCKS);
}

static void heap_mark_block_taken(struct heap *heap, int start_block, int total_block)
{
    int end_block = start_block + total_block - 1;
    HEAP_BLOCK_TABLE_ENTRY entry = HEAP_BLOCK_TABLE_ENTRY_TAKEN | HEAP_BLOCK_IS_FIRST;
    if (total_block > 1)
    {
        entry |= HEAP_BLOCK_HAS_NEXT;
    }
    for (int i = start_block; i <= end_block; i++)
    {
        heap->table->entries[i] = entry;
        entry = HEAP_BLOCK_TABLE_ENTRY_TAKEN;
        if (i != end_block - 1)
        {
            entry |= HEAP_BLOCK_HAS_NEXT;
        }
    }
}

void *heap_malloc_blocks(struct heap *heap, uint32_t total_block)
{
    void *addr = NULL;

    int start_block = heap_get_start_block(heap, total_block);
    if (start_block < 0)
    {
        goto out;
    }

    addr = heap_block_to_address(heap, start_block);

    heap_mark_block_taken(heap, start_block, total_block);

out:
    return addr;
}

void heap_mark_block_free(struct heap *heap, int start_block)
{
    struct heap_table *table = heap->table;
    HEAP_BLOCK_TABLE_ENTRY start = table->entries[start_block];
    if (!(start & HEAP_BLOCK_IS_FIRST))
    {
        return;
    }
    for (int i = start_block; i <= (int)table->total; i++)
    {
        HEAP_BLOCK_TABLE_ENTRY entry = table->entries[i];
        heap->table->entries[i] = HEAP_BLOCK_TABLE_ENTRY_FREE;
        if (!(entry & HEAP_BLOCK_HAS_NEXT))
        {
            break;
        }
    }
}

int heap_address_to_block(struct heap *heap, void *addr)
{
    return ((int)(addr - heap->saddr)) / HEAP_SIZE_BLOCKS;
}

void *heap_malloc(struct heap *heap, size_t size)
{
    size_t aligned_size = heap_align_value_to_upper(size);
    uint32_t total_block = aligned_size / HEAP_SIZE_BLOCKS;
    return heap_malloc_blocks(heap, total_block);
}

void heap_free(struct heap *heap, void *ptr)
{
    if (ptr == NULL)
    {
        return;
    }
    heap_mark_block_free(heap, heap_address_to_block(heap, ptr));
}

int get_number_free_block(struct heap *heap)
{
    int free = 0;
    for (size_t i = 0; i < heap->table->total; i++)
    {
        if (heap->table->entries[i] == HEAP_BLOCK_TABLE_ENTRY_FREE)
        {
            free++;
        }
    }
    return free;
}