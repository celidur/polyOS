#include <os/int80/misc.h>
#include <os/kernel.h>

void *int80h_command19_reboot(struct interrupt_frame *frame){
    reboot();
    return 0;
}
