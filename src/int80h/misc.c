#include <os/int80/misc.h>
#include <os/kernel.h>

void *int80h_command19_reboot(struct interrupt_frame *frame){
    reboot();
    return 0;
}

void *int80h_command20_shutdown(struct interrupt_frame *frame){
    shutdown();
    return 0;
}