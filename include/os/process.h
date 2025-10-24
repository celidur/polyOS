#ifndef PROCESS_H
#define PROCESS_H

struct command_argument{
    char argument[512];
    struct command_argument* next;
};

struct process_argument{
    int argc;
    char** argv;
};

#endif