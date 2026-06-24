#ifndef POLYOS_STDLIB_H
#define POLYOS_STDLIB_H

#include "types.h"

void* malloc(size_t size);
void free(void *ptr);
int brk(void *addr);
void *sbrk(intptr_t increment);
char* itoa(int i);
char* hex(uint32_t i);
extern char **environ;
char *getenv(const char *name);
int setenv(const char *name, const char *value, int overwrite);
int unsetenv(const char *name);

#endif
