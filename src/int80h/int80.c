#include <os/int80/int80.h>
#include <os/idt.h>
#include <os/int80/misc.h>
#include <os/int80/io.h>
#include <os/int80/heap.h>
#include <os/int80/process.h>

void int80h_register_commands()
{
    int80h_register_command(SYSTEM_COMMAND0_SUM, int80h_command0_sum);
    int80h_register_command(SYSTEM_COMMAND1_PRINT, int80h_command1_print);
    int80h_register_command(SYSTEM_COMMAND2_GETKEY, int80h_command2_getkey);
    int80h_register_command(SYSTEM_COMMAND3_PUTCHAR, int80h_command3_putchar);
    int80h_register_command(SYSTEM_COMMAND4_MALLOC, int80h_command4_malloc);
    int80h_register_command(SYSTEM_COMMAND5_FREE, int80h_command5_free);
    int80h_register_command(SYSTEM_COMMAND6_process_load_start, int80h_command6_process_load_start);
    int80h_register_command(SYSTEM_COMMAND7_INVOKE_SYSTEM_COMMAND, int80h_command7_invoke_system_command);
    int80h_register_command(SYSTEM_COMMAND8_GET_PROCESS_ARGUMENTS, int80h_command8_get_program_arguments);
    int80h_register_command(SYSTEM_COMMAND9_EXIT, int80h_command9_exit);
    int80h_register_command(SYSTEM_COMMAND10_PRINT_MEMORY, int80h_command10_print_memory);
    int80h_register_command(SYSTEM_COMMAND11_REMOVE_CHAR, int80h_command11_remove_last_char);
    int80h_register_command(SYSTEM_COMMAND12_CLEAR_SCREEN, int80h_command12_clear_screen);
}