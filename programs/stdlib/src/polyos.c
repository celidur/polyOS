#include "polyos.h"
#include "string.h"

int polyos_getkeyblock(){
    int val = 0;
    do
    {
        val = polyos_getkey();
    } while (val == 0);
    return val;
}

void polyos_terminal_readline(char* out, int max, bool output_while_typing)
{
    int i = 0;
    for (i = 0; i < max; i++)
    {
        char key = polyos_getkeyblock();

        // Carriage return means we're done
        if (key == 13){
            break;
        }

        if (output_while_typing){
            polyos_putchar(key);
        }

        // Backspace
        if (key == 0x08 && i > 0){
            out[i-1] = '\0';
            i -= 2;
            continue;
        }
        out[i] = key;
    }
    // Null terminate
    out[i] = '\0';
}

struct command_argument* polyos_parse_command(char *command, int max){
    struct command_argument* root_command = NULL;
    char scommand[1025];
    if (max >= (int) sizeof(scommand)){
        return NULL;
    }

    strncpy(scommand, command, sizeof(scommand));
    char* token = strtok(scommand, " ");
    if (!token){
        return NULL;
    }

    root_command = polyos_malloc(sizeof(struct command_argument));
    if (!root_command){
        return NULL;
    }

    strncpy(root_command->argument, token, sizeof(root_command->argument));
    root_command->next = NULL;

    struct command_argument* current_command = root_command;
    token = strtok(NULL, " ");
    while(token){
        struct command_argument* next_command = polyos_malloc(sizeof(struct command_argument));
        if (!next_command){
            return root_command;
        }
        strncpy(next_command->argument, token, sizeof(next_command->argument));
        next_command->next = NULL;
        current_command->next = next_command;
        current_command = next_command;
        token = strtok(NULL, " ");
    }

    return root_command;
}

int polyos_system_run(const char *command){
    char buff[1024];
    strncpy(buff, command, sizeof(buff));
    struct command_argument* root_command = polyos_parse_command(buff, sizeof(buff));
    if (!root_command){
        return -1;
    }
    return polyos_system(root_command);
}