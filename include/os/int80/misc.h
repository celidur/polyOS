#ifndef INT80_MISC_H
#define INT80_MISC_H

struct interrupt_frame;
void *int80h_command19_reboot(struct interrupt_frame *frame);

#endif