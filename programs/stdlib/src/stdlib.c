#include <stdlib.h>
#include "polyos.h"

void* malloc(size_t size)
{
    return polyos_malloc(size);
}

void free(void* ptr)
{
    polyos_free(ptr);
}

char* itoa(int i){
    static char str[12];
    int loc = 11;
    str[loc] = '\0';
    char neg = 1;
    if (i >= 0){
        neg = 0;
        i = -i;
    }

    while (i){
        str[--loc] = '0' - (i % 10);
        i /= 10;
    }

    if (loc == 11){
        str[--loc] = '0';
    }
    if (neg){
        str[--loc] = '-';
    }
    return &str[loc];
}