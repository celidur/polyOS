#ifndef MEMORY_H
#define MEMORY_H

#include <os/types.h>

void *memset(void *ptr, int c, size_t size);

int memcmp(void *ptr1, void *ptr2, size_t size);

void *memcpy(void *destination, const void *source, size_t num);

void *memmove(void *dest, const void *src, size_t n);

#endif