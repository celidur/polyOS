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

void *memcpy(void *restrict destination, const void *restrict source, size_t num)
{
    char *c_destination = (char *)destination;
    const char *c_source = (char *)source;
    if (c_destination == c_source || num == 0)
    {
        return destination;
    }

    if (c_destination < c_source)
    {
        for (size_t i = 0; i < num; i++)
        {
            c_destination[i] = c_source[i];
        }
    }
    else
    {
        for (size_t i = num; i > 0; i--)
        {
            c_destination[i - 1] = c_source[i - 1];
        }
    }
    return destination;
}

void *memmove(void *restrict destination, const void *restrict source, size_t num)
{
    char *c_destination = (char *)destination;
    const char *c_source = (char *)source;
    if (c_destination == c_source || num == 0)
    {
        return destination;
    }

    if (c_destination < c_source)
    {
        for (size_t i = 0; i < num; i++)
        {
            c_destination[i] = c_source[i];
        }
    }
    else
    {
        for (size_t i = num; i > 0; i--)
        {
            c_destination[i - 1] = c_source[i - 1];
        }
    }
    return destination;
}