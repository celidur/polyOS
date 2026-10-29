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


void *calloc(size_t count,size_t size);
void *realloc(void *ptr,size_t size);
int posix_memalign(void **out,size_t alignment,size_t size);
void *aligned_alloc(size_t alignment,size_t size);
void *memalign(size_t alignment,size_t size);

#endif
