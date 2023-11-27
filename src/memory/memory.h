#ifndef MEMORY_H
#define MEMORY_H

#include <stddef.h>

void *memset(void *ptr, int c, size_t size);

int memcmp(void *ptr1, void *ptr2, size_t size);

void *memcpy(void *restrict destination, const void *restrict source, size_t num);

void *memmove(void *destination, const void *source, size_t num);

#endif