#ifndef POLYOS_H
#define POLYOS_H

#include <stddef.h>
#include <stdbool.h>

struct command_argument {
    char argument[512];
    struct command_argument *next;
};

struct command_argument* polyos_parse_command(char *command, int max);

struct process_arguments {
    int argc;
    char **argv;
};

void print(char *str);
int polyos_getkey();
int polyos_getkeyblock();
void polyos_terminal_readline(char* out, int max, bool output_while_typing);
void* polyos_malloc(size_t size);
void polyos_free(void* ptr);
void polyos_putchar(char c);
void polyos_process_load_start(const char *filename);
void polyos_process_get_args(struct process_arguments *args);
int polyos_system(struct command_argument *args);
int polyos_system_run(const char *command);
void polyos_exit();
void print_memory();

#endif