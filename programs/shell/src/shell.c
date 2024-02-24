#include "shell.h"
#include "stdio.h"
#include "stdlib.h"
#include "polyos.h"
#include "string.h"

int main(int argc, char **argv)
{
    printf("PolyOS v1.0.0\n");
    while (1){
        printf("> ");
        char buffer[1024];
        polyos_terminal_readline(buffer,sizeof(buffer), true);
        printf("\n");
        if (buffer[0] == '\0'){
            continue;
        }
        if (strncmp(buffer, "memory", 7) == 0){
            print_memory();
        }else if (strncmp(buffer, "exit", 5) == 0){
            break;
        }else if (strncmp(buffer, "malloc", 7) == 0){
            char *ptr = malloc(4096*4096);
            printf("malloc: %p\n", ptr);
        } else if (strncmp(buffer, "clear", 6) == 0){
            clear_screen();
        }
        
        else if (polyos_system_run(buffer) < 0){
            printf("Command not found\n");
        }
    }

    return 0;
}