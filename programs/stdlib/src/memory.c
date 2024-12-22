#include "memory.h"

void* memset(void* ptr, int c, size_t size){
    char* p = (char*)ptr;
    for (size_t i = 0; i < size; i++){
        p[i] = (char)c;
    }
    return ptr;
}

int memcmp(const void* ptr1, const void* ptr2, size_t size){
    const char* p1 = (const char*)ptr1;
    const char* p2 = (const char*)ptr2;
    for (size_t i = 0; i < size; i++){
        if (p1[i] != p2[i]){
            return p1[i] - p2[i];
        }
    }
    return 0;
}

void* memcpy(void* dest, const void* src, size_t size){
    char* d = (char*)dest;
    const char* s = (const char*)src;
    for (size_t i = 0; i < size; i++){
        d[i] = s[i];
    }
    return dest;
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