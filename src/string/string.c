#include "string.h"

int strlen(const char *str)
{
    int len = 0;
    while (str[len])
    {
        len++;
    }
    return len;
}

int strnlen(const char *str, int max)
{
    int len = 0;
    while (str[len] && len < max)
    {
        len++;
    }
    return len;
}

bool isdigit(char c)
{
    return c >= '0' && c <= '9';
}

int tonumericdigit(char c)
{
    return c - '0';
}

char *strcpy(char *dest, const char *src)
{
    int i = 0;
    while (src[i])
    {
        dest[i] = src[i];
        i++;
    }
    dest[i] = 0;
    return dest;
}

char tolower(char c)
{
    if (c >= 'A' && c <= 'Z')
    {
        return c - 'A' + 'a';
    }
    return c;
}

int strncmp(const char *s1, const char *s2, int n)
{
    unsigned char c1, c2;
    while (n--)
    {
        c1 = *s1++;
        c2 = *s2++;
        if (c1 != c2)
        {
            return c1 - c2;
        }
        if (!c1)
        {
            return 0;
        }
    }
    return 0;
}

int istrncmp(const char *s1, const char *s2, int n)
{
    unsigned char c1, c2;
    while (n--)
    {
        c1 = tolower(*s1++);
        c2 = tolower(*s2++);
        if (c1 != c2)
        {
            return c1 - c2;
        }
        if (!c1)
        {
            return 0;
        }
    }
    return 0;
}

int strlen_terminator(const char *str, char terminator)
{
    int len = 0;
    while (str[len] && str[len] != terminator)
    {
        len++;
    }
    return len;
}