#ifndef STRING_H
#define STRING_H

#include <stdbool.h>

int strlen(const char *str);
int strnlen(const char *str, int max);
bool isdigit(char c);
int tonumericdigit(char c);
char *strcpy(char *dest, const char *src);
char *strncpy(char *dest, const char *src, int n);
char tolower(char c);
int strncmp(const char *s1, const char *s2, int n);
int istrncmp(const char *s1, const char *s2, int n);
int strlen_terminator(const char *str, char terminator);

#endif