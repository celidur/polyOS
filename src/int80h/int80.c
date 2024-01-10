#include "int80.h"
#include "idt/idt.h"
#include "misc.h"
#include "io.h"
#include "heap.h"
#include "process.h"

void int80h_register_commands()
{
    int80h_register_command(SYSTEM_COMMAND0_SUM, int80h_command0_sum);
    int80h_register_command(SYSTEM_COMMAND1_PRINT, int80h_command1_print);
    int80h_register_command(SYSTEM_COMMAND2_GETKEY, int80h_command2_getkey);
    int80h_register_command(SYSTEM_COMMAND3_PUTCHAR, int80h_command3_putchar);
    int80h_register_command(SYSTEM_COMMAND4_MALLOC, int80_command4_malloc);
    int80h_register_command(SYSTEM_COMMAND5_FREE, int80_command5_free);
    int80h_register_command(SYSTEM_COMMAND6_process_load_start, int80_command6_process_load_start);
}