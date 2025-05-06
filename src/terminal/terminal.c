#include <os/terminal.h>
#include <os/string.h>
#include <os/types.h>
#include <os/io.h>
#include <os/types.h>
#include <stdarg.h>
#include <os/memory.h>

#define MAX_BUFFER 1024

int serial_write(const char *buf);

static char* itoa(s64 i) {
    static char str[22];
    memset(str, '0', 22);
    int loc = 21;
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

    if (loc == 21){
        str[--loc] = '0';
    }
    if (neg){
        str[--loc] = '-';
    }
    return &str[loc];
}

static char* hex(uint32_t i){
    static char str[12];
    int loc = 11;
    str[loc] = '\0';
    while (i){
        int rem = i % 16;
        if (rem < 10){
            str[--loc] = '0' + rem;
        } else {
            str[--loc] = 'A' + (rem - 10);
        }
        i /= 16;
    }

    if (loc == 11){
        str[--loc] = '0';
    }
    
    return &str[loc];

}

int printf(const char *fmt, ...){
    va_list ap;
    const char* p;
    char* sval;
    int ival;
    char buff[MAX_BUFFER + 1];
    int i=0;
    
    va_start(ap, fmt);
    for (p = fmt; *p; p++){
        if (i >= MAX_BUFFER){
            buff[i] = '\0';
            print(buff);
            serial_write(buff);
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
                    if (i >= MAX_BUFFER){
                        buff[i] = '\0';
                        print(buff);
                        serial_write(buff);
                        i = 0;
                    }
                    buff[i++] = sval[j];
                }
                break;
            case 's':
                sval = va_arg(ap, char*);
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER){
                        buff[i] = '\0';
                        print(buff);
                        serial_write(buff);
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
                sval = hex(va_arg(ap, uint32_t));
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER){
                        buff[i] = '\0';
                        print(buff);
                        serial_write(buff);
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
    serial_write(buff);

    return 0;
}

int serial_printf(const char *fmt, ...){
    va_list ap;
    const char* p;
    char* sval;
    u32 ival;
    u64 lval;
    char buff[MAX_BUFFER + 1];
    int i=0;

    va_start(ap, fmt);
    for (p = fmt; *p; p++){
        if (i >= MAX_BUFFER){
            buff[i] = '\0';
            serial_write(buff);
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
                    if (i >= MAX_BUFFER){
                        buff[i] = '\0';
                        serial_write(buff);
                        i = 0;
                    }
                    buff[i++] = sval[j];
                }
                break;
            case 'l':
                lval = va_arg(ap, u64);
                sval = itoa(lval);
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER){
                        buff[i] = '\0';
                        serial_write(buff);
                        i = 0;
                    }
                    buff[i++] = sval[j];
                }
                break;
            case 's':
                sval = va_arg(ap, char*);
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER){
                        buff[i] = '\0';
                        serial_write(buff);
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
                sval = hex(va_arg(ap, uint32_t));
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER){
                        buff[i] = '\0';
                        serial_write(buff);
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
    return serial_write(buff);
}