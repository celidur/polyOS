#ifndef INT80_H
#define INT80_H

enum SystemCommands
{
    SYSTEM_COMMAND0_SUM,
    SYSTEM_COMMAND1_PRINT,
};

void int80h_register_commands();

#endif