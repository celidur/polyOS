#ifndef POLYOS_H
#define POLYOS_H

#include <stddef.h>
#include <stdbool.h>

void print(char *str);
int polyos_getkey();
int polyos_getkeyblock();
void polyos_terminal_readline(char* out, int max, bool output_while_typing);
void* polyos_malloc(size_t size);
void polyos_free(void* ptr);
void polyos_putchar(char c);

#endif