#include "stdio.h"
#include "polyos.h"
#include "stdlib.h"
#include <stdarg.h>

#define MAX_BUFFER 1024

int putchar(int c)
{
    polyos_putchar(c);
    return 0;
}

int printf(const char *fmt, ...){
    va_list ap;
    const char* p;
    char* sval;
    int ival;
    char buff[MAX_BUFFER];
    int i=0;

    va_start(ap, fmt);
    for (p = fmt; *p; p++){
        if (i >= MAX_BUFFER - 1){
            buff[i] = '\0';
            print(buff);
            i = 0;
        }
        if (*p != '%'){
            buff[i++] = *p;
            continue;
        }
        switch (*++p){
            case 'd':
                ival = va_arg(ap, int);
                sval = itoa(ival);
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER - 1){
                        buff[i] = '\0';
                        print(buff);
                        i = 0;
                    }
                    buff[i++] = sval[j];
                }
                break;
            case 's':
                sval = va_arg(ap, char*);
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER - 1){
                        buff[i] = '\0';
                        print(buff);
                        i = 0;
                    }
                    buff[i++] = sval[j];
                }
                break;
            case 'c':
                ival = va_arg(ap, int);
                buff[i++] = ival;
                break;
            case 'x':
                uint32_t xval = va_arg(ap, uint32_t);
                sval = hex(xval);
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER - 1){
                        buff[i] = '\0';
                        print(buff);
                        i = 0;
                    }
                    buff[i++] = sval[j];
                }
                break;
            default:
                buff[i++] = *p;
                break;
        }
    }

    va_end(ap);

    buff[i] = '\0';
    print(buff);

    return 0;
}