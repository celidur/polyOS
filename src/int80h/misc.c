#include "misc.h"
#include "idt/idt.h"
#include "kernel.h"

void *int80h_commando_sum(struct interrupt_frame *frame)
{
    return (void *)5;
}