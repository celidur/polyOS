#ifndef INT80_H
#define INT80_H

enum SystemCommands
{
    SYSTEM_COMMAND0_SUM,
    SYSTEM_COMMAND1_PRINT,
    SYSTEM_COMMAND2_GETKEY,
    SYSTEM_COMMAND3_PUTCHAR,
    SYSTEM_COMMAND4_MALLOC,
    SYSTEM_COMMAND5_FREE,
    SYSTEM_COMMAND6_process_load_start,
    SYSTEM_COMMAND7_INVOKE_SYSTEM_COMMAND,
    SYSTEM_COMMAND8_GET_PROCESS_ARGUMENTS,
    SYSTEM_COMMAND9_EXIT,
    SYSTEM_COMMAND10_PRINT_MEMORY,
    SYSTEM_COMMAND11_REMOVE_CHAR,
    SYSTEM_COMMAND12_CLEAR_SCREEN,
};

void int80h_register_commands();

#endif