#ifndef IO_H
#define IO_H

struct interrupt_frame;
void *int80h_command1_print(struct interrupt_frame *frame);

#endif
