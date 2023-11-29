#include "int80.h"
#include "idt/idt.h"
#include "misc.h"

void int80h_register_commands()
{
    int80h_register_command(SYSTEM_COMMANDO_SUM, int80h_commando_sum);
}