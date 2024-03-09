#include "stdio.h"
#include <stddef.h>

int main (int argc, char** argv) {
    printf("argc: %d\n", argc);
    for (int i = 0; i < argc; i++){
        printf("argv[%d]: %s\n", i, argv[i]);
    }
    int a = 0;
    for (size_t i = 0; i < 100000000; i++)
    {
        a *= i;
    }
    
    (void)a;
    
    printf("program end\n");
    
    return 0;
}