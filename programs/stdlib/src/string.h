#ifndef POLYOS_STRING_H
#define POLYOS_STRING_H

#include <stdbool.h>

char tolower(char c);
int strlen(const char* str);
int strnlen(const char* str, int max);
int strnlen_terminator(const char* str, int max, char terminator);
int istrncmp(const char* str1, const char* str2, int max);
int strncmp(const char* str1, const char* str2, int max);
char* strcpy(char* dest, const char* src);
char* strncpy(char* dest, const char* src, int max);
bool isdigit(char c);
int tonumericdigit(char c);
char* strtok(char* str, const char* delim);

#endif