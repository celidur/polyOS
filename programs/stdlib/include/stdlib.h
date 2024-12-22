#ifndef POLYOS_STDLIB_H
#define POLYOS_STDLIB_H

#include "types.h"

void* malloc(size_t size);
void free(void *ptr);
char* itoa(int i);
char* hex(uint32_t i);

#endif