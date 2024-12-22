#ifndef POLYOS_MEMORY_H
#define POLYOS_MEMORY_H

#include <types.h>

void* memset(void* ptr, int c, size_t size);
int memcmp(const void* ptr1, const void* ptr2, size_t size);
void* memcpy(void* dest, const void* src, size_t size);
void *memmove(void *dest, const void *src, size_t n);

#endif