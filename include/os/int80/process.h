#ifndef INT80_PROCESS_H
#define INT80_PROCESS_H

struct interrupt_frame;
void* int80h_command6_process_load_start(struct interrupt_frame *frame);
void* int80h_command8_get_program_arguments(struct interrupt_frame *frame);
void* int80h_command7_invoke_system_command(struct interrupt_frame *frame);
void* int80h_command9_exit(struct interrupt_frame *frame);

#endif