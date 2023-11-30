#include "int80.h"
#include "idt/idt.h"
#include "misc.h"
#include "io.h"

void int80h_register_commands()
{
    int80h_register_command(SYSTEM_COMMAND0_SUM, int80h_command0_sum);
    int80h_register_command(SYSTEM_COMMAND1_PRINT, int80h_command1_print);
}