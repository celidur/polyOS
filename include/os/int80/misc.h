#ifndef INT80_MISC_H
#define INT80_MISC_H

struct interrupt_frame;
void *int80h_command19_reboot(struct interrupt_frame *frame);
void *int80h_command20_shutdown(struct interrupt_frame *frame);

#endif