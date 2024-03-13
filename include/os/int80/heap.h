#ifndef INT80_HEAP_H
#define INT80_HEAP_H

struct interrupt_frame;
void* int80h_command4_malloc(struct interrupt_frame *frame);
void* int80h_command5_free(struct interrupt_frame *frame);
void* int80h_command10_print_memory(struct interrupt_frame *frame);

#endif