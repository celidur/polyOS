#include <os/task.h>
#include <os/kernel.h>
#include <os/status.h>
#include <os/kheap.h>
#include <os/memory.h>
#include <os/idt.h>
#include <os/paging.h>
#include <os/string.h>

// int copy_string_from_task(uint32_t *directory, void *virt, void *phys, int max)
// {
//     if (max >= PAGING_PAGE_SIZE)
//         return -EINVARG;

//     char *buffer = kpalloc(max);
//     if (!buffer)
//         return -ENOMEM;

//     uint32_t old_entry = paging_get(directory, buffer);

//     paging_map(directory, buffer, buffer, PAGING_IS_PRESENT | PAGING_ACCESS_FROM_ALL | PAGING_IS_WRITABLE);
//     paging_switch(directory);
//     strncpy(buffer, virt, max);
//     kernel_page();

//     if (paging_set(directory, buffer, old_entry) < 0)
//     {
//         kfree(buffer);
//         return -EIO;
//     }

//     strncpy(phys, buffer, max);
//     kfree(buffer);
//     return 0;
// }

// int copy_string_to_task(u32 *directory, void* buff, void* virt, u32 size)
// {
//     u32 size_remaining = size;
//     u32 size_to_copy = 0;
//     u32 offset = 0;
//     u32 old_entry = 0;

//     while(size_remaining > 0){
//         size_to_copy = size_remaining > PAGING_PAGE_SIZE ? PAGING_PAGE_SIZE : size_remaining;
//         void *ptr = (void* )((u32)(buff) + offset);

//         old_entry = paging_get(directory, paging_align_to_lower_page(ptr));
//         paging_map(directory, ptr, ptr, PAGING_IS_PRESENT | PAGING_ACCESS_FROM_ALL | PAGING_IS_WRITABLE);
//         paging_switch(directory);
//         strncpy(virt, ptr, size_to_copy);
//         kernel_page();

//         if (paging_set(directory, paging_align_to_lower_page(ptr), old_entry) < 0)
//         {
//             return -EIO;
//         }

//         size_remaining -= size_to_copy;
//         offset += size_to_copy;
//     }
//     return 0;
// }

// void* task_virtual_address_to_physical(u32 *directory, void* virtual_address){
//     return paging_get_physical_address(directory, virtual_address);
// }