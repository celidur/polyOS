#include "kheap.h"
#include "heap.h"
#include "config.h"
#include "kernel.h"
#include "memory/memory.h"

struct heap kernel_heap;
struct heap_table kernel_heap_table;

void kheap_init()
{
    int total_table_entries = HEAP_SIZE_BYTES / HEAP_SIZE_BLOCKS;
    kernel_heap_table.entries = (HEAP_BLOCK_TABLE_ENTRY *)HEAP_TABLE_ADDRESS;
    kernel_heap_table.total = total_table_entries;

    void *end = (void *)HEAP_ADDRESS + HEAP_SIZE_BYTES;
    int res = heap_create(&kernel_heap, (void *)HEAP_ADDRESS, end, &kernel_heap_table);
    if (res < 0)
    {
        kernel_panic("Failed to create kernel heap\n");
    }
}

void *kmalloc(size_t size)
{
    void *ptr = heap_malloc(&kernel_heap, size);
    return ptr;
}

void kfree(void *ptr)
{
    heap_free(&kernel_heap, ptr);
}

void *kzalloc(size_t size)
{
    void *ptr = heap_malloc(&kernel_heap, size);
    if (ptr)
    {
        memset(ptr, 0, size);
    }
    return ptr;
}