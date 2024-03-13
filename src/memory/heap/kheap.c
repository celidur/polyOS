#include <os/kheap.h>
#include <os/heap.h>
#include <os/config.h>
#include <os/kernel.h>
#include <os/memory.h>
#include <os/terminal.h>

static struct heap kernel_heap;
static struct heap_table kernel_heap_table;

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

void print_data(uint32_t data){
    if (data > 1024*1024*1024){
        printf("%d GB", data/(1024*1024*1024));
    } else if (data > 1024*1024){
        printf("%d MB", data/(1024*1024));
    } else if (data > 1024){
        printf("%d KB", data/1024);
    } else {
        printf("%d B", data);
    }
}

void print_memory(){
    uint32_t block_free = get_number_free_block(&kernel_heap);
    uint32_t free_memory = block_free * HEAP_SIZE_BLOCKS;
    uint32_t total_memory = HEAP_SIZE_BYTES;
    uint32_t used_memory = total_memory - free_memory;
    uint32_t tmp = total_memory/100;
    
    float percentage_free = free_memory/tmp;

    printf("Total memory: ");
    print_data(total_memory);
    printf("\n");

    printf("Used memory: ");
    print_data(used_memory);
    printf(" (%d%%)\n", (int)(100-percentage_free));

    printf("Free memory: ");
    print_data(free_memory);
    printf(" (%d%%)\n", (int)percentage_free);

}