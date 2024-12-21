#include <os/memory.h>
#include <os/kheap.h>

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

void *memcpy(void *destination, const void *source, size_t num)
{
    char *c_destination = (char *)destination;
    const char *c_source = (char *)source;
    while (num--)
    {
        *c_destination++ = *c_source++;
    }
    return destination;
}

void *memmove(void *dest, const void *src, size_t n) {
    unsigned char *d = (unsigned char *)dest;
    const unsigned char *s = (const unsigned char *)src;
    if (d < s) {
        for (size_t i = 0; i < n; i++) {
            d[i] = s[i];
        }
    } else {
        for (size_t i = n; i > 0; i--) {
            d[i - 1] = s[i - 1];
        }
    }
    return dest;
}
