#ifndef IO_H
#define IO_H

struct interrupt_frame;
void *int80h_command1_print(struct interrupt_frame *frame);
void *int80h_command2_getkey(struct interrupt_frame *frame);
void *int80h_command3_putchar(struct interrupt_frame *frame);
void *int80h_command11_remove_last_char(struct interrupt_frame *frame);
void *int80h_command12_clear_screen(struct interrupt_frame *frame);

#endif
