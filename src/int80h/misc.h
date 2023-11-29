#ifndef INT80H_MISC_H
#define INT80H_MISC_H

struct interrupt_frame;
void *int80h_commando_sum(struct interrupt_frame *frame);

#endif