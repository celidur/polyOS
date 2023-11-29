#ifndef INT80_H
#define INT80_H

enum SystemCommands
{
    SYSTEM_COMMANDO_SUM,
};

void int80h_register_commands();

#endif