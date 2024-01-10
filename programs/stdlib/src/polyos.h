#ifndef POLYOS_H
#define POLYOS_H

#include <stddef.h>

void print(char *str);
int getkey();
void* polyos_malloc(size_t size);
void polyos_free(void* ptr);

#endif