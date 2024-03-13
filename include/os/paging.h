#ifndef PAGING_H
#define PAGING_H

#include <os/config.h>
#include <os/types.h>

#define PAGING_CACHE_DISABLED 0b00010000
#define PAGING_WRITE_THROUGH 0b00001000
#define PAGING_ACCESS_FROM_ALL 0b00000100
#define PAGING_IS_WRITABLE 0b00000010
#define PAGING_IS_PRESENT 0b00000001

typedef u32 page_t;
page_t *paging_new_4gb(u8 flags);
void paging_switch(page_t *directory);
void enable_paging();
int paging_map_to(page_t *directory, void *virt, void *phys, void *phys_end, u8 flags);
int paging_map_range(page_t *directory, void *virt, void *phys, int count, u8 flags);
int paging_map(page_t *directory, void *virt, void *phys, u8 flags);
void *paging_align_address(void *addr);
void paging_free_4gb(page_t *chunk);
int paging_set(u32 *directory, void *virtual_addr, u32 value);
bool paging_is_aligned(void *addr);
u32 paging_get(u32 *directory, void *virtual_addr);
void* paging_align_to_lower_page(void* addr);
void* paging_get_physical_address(u32* directory, void* virtual_address);
void print_paging_info(u32* directory);
#endif