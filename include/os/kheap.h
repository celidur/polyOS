#ifndef KHEAP_H
#define KHEAP_H

#include <os/types.h>

void *kmalloc(size_t size);
void *kzalloc(size_t size);
void *kpalloc(size_t size);
void kfree(void *ptr);
void print_memory();
void serial_print_memory();

#endif