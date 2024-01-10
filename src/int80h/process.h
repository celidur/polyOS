#ifndef INT80_PROCESS_H
#define INT80_PROCESS_H

struct interrupt_frame;
void* int80_command6_process_load_start(struct interrupt_frame *frame);

#endif