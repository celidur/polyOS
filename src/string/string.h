#ifndef STRING_H
#define STRING_H

#include <stdbool.h>

int strlen(const char *str);
int strnlen(const char *str, int max);
bool isdigit(char c);
int tonumericdigit(char c);

#endif