#include "memory.h"

void *memset(void *ptr, int c, size_t size)
{
    char *c_ptr = (char *)ptr;
    for (size_t i = 0; i < size; i++)
    {
        c_ptr[i] = (char)c;
    }
    return ptr;
}

int memcmp(void *ptr1, void *ptr2, size_t size)
{
    char *c_ptr1 = (char *)ptr1;
    char *c_ptr2 = (char *)ptr2;
    while (size--)
    {
        if (*c_ptr1++ != *c_ptr2++)
        {
            return c_ptr1[-1] < c_ptr2[-1] ? -1 : 1;
        }
    }
    return 0;
}