#include "shell.h"
#include "stdio.h"
#include "stdlib.h"
#include "polyos.h"

int main(int argc, char **argv)
{
    printf("PolyOS v1.0.0\n");
    while (1){
        printf("> ");
        char buffer[1024];
        polyos_terminal_readline(buffer,sizeof(buffer), true);
        printf("\n");
    }
    
    return 0;
}