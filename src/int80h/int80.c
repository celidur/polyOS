#include <os/int80/int80.h>
#include <os/idt.h>
#include <os/int80/misc.h>
#include <os/int80/io.h>
#include <os/int80/heap.h>
#include <os/int80/process.h>
#include <os/int80/file.h>

void int80h_register_commands()
{
    // int80h_register_command(SYSTEM_COMMAND0_SUM, int80h_command0_sum);
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
    int80h_register_command(SYSTEM_COMMAND13_OPEN_FILE, int80h_command13_fopen);
    int80h_register_command(SYSTEM_COMMAND14_READ_FILE, int80h_command14_fread);
    int80h_register_command(SYSTEM_COMMAND15_WRITE_FILE, int80h_command15_fwrite);
    int80h_register_command(SYSTEM_COMMAND16_SEEK_FILE, int80h_command16_fseek);
    int80h_register_command(SYSTEM_COMMAND17_STAT_FILE, int80h_command17_fstat);
    int80h_register_command(SYSTEM_COMMAND18_CLOSE_FILE, int80h_command18_fclose);
}