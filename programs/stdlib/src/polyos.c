#include "polyos.h"

int polyos_getkeyblock(){
    int val = 0;
    do
    {
        val = polyos_getkey();
    } while (val == 0);
    return val;
}

void polyos_terminal_readline(char* out, int max, bool output_while_typing)
{
    int i = 0;
    for (i = 0; i < max; i++)
    {
        char key = polyos_getkeyblock();

        // Carriage return means we're done
        if (key == 13){
            break;
        }

        if (output_while_typing){
            polyos_putchar(key);
        }

        // Backspace
        if (key == 0x08 && i > 0){
            out[i-1] = '\0';
            i -= 2;
            continue;
        }
        out[i] = key;
    }
    // Null terminate
    out[i] = '\0';
}