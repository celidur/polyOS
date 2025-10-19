#include <os/string.h>

int strlen(const char *str)
{
    int len = 0;
    while (str[len])
    {
        len++;
    }
    return len;
}

char *strncpy(char *dest, const char *src, int n)
{
    int i = 0;
    while (src[i] && i < n - 1)
    {
        dest[i] = src[i];
        i++;
    }
    dest[i] = 0;
    return dest;
}