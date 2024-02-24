#include "string.h"
#include "stdio.h"

char tolower(char c){
    if (c >= 'A' && c <= 'Z'){
        return c - 'A' + 'a';
    }
    return c;
}

int strlen(const char* str){
    int i = 0;
    while (str[i] != 0){
        i++;
    }
    return i;
}

int strnlen(const char* str, int max){
    int i = 0;
    while (str[i] != 0 && i < max){
        i++;
    }
    return i;
}

int strnlen_terminator(const char* str, int max, char terminator){
    int i = 0;
    while (str[i] != 0 && i < max && str[i] != terminator){
        i++;
    }
    return i;
}

int istrncmp(const char* str1, const char* str2, int max){
    int i = 0;
    while (str1[i] != 0 && str2[i] != 0 && i < max){
        char c1 = tolower(str1[i]);
        char c2 = tolower(str2[i]);
        if (c1 != c2){
            return c1 - c2;
        }
        i++;
    }
    return 0;
}

int strncmp(const char* str1, const char* str2, int max){
    int i = 0;
    while (str1[i] != 0 && str2[i] != 0 && i < max){
        if (str1[i] != str2[i]){
            return str1[i] - str2[i];
        }
        i++;
    }
    return 0;
}

char* strcpy(char* dest, const char* src){
    int i = 0;
    while (src[i] != 0){
        dest[i] = src[i];
        i++;
    }
    dest[i] = 0;
    return dest;
}

char* strncpy(char* dest, const char* src, int max){
    int i = 0;
    while (src[i] != 0 && i < max){
        dest[i] = src[i];
        i++;
    }
    dest[i] = 0;
    return dest;
}

bool isdigit(char c){
    return c >= '0' && c <= '9';
}

int tonumericdigit(char c){
    return c - '0';
}

char* sp = (char*)1;
char* strtok(char* str, const char* delimiters)
{
    int i = 0;
    int len = strlen(delimiters);
    if (!str && !sp)
        return 0;

    
    if (!sp || sp == (char*)1)
    {
        sp = str;
    }


    char* p_start = sp;
    while(1)
    {
        for (i = 0; i < len; i++)
        {
            if(*p_start == delimiters[i])
            {
                p_start++;
                break;
            }
        }

        if (i == len)
        {
            sp = p_start;
            break;
        }
    }

    if (*sp == '\0')
    {
        sp = 0;
        return sp;
    }

    // Find end of substring
    while(*sp != '\0')
    {
        for (i = 0; i < len; i++)
        {
            if (*sp == delimiters[i])
            {
                *sp = '\0';
                break;
            }
        }

        sp++;
        if (i < len)
            break;
    }

    return p_start;
}