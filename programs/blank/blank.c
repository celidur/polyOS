#include "stdlib.h"
#include "stdio.h"
#include "polyos.h"
#include "string.h"

int main (int argc, char** argv) {
    char str[] = "hello world";
    struct command_argument* root_command = polyos_parse_command(str, sizeof(str));
    if (!root_command){
        printf("Failed to parse command\n");
        while (1){}
    }

    struct command_argument* current_command = root_command;
    while (current_command){
        printf("Argument: %s\n", current_command->argument);
        current_command = current_command->next;
    }
    while (1){}
    return 0;
}