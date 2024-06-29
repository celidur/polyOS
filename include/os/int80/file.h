#ifndef INT80_FILE_H
#define INT80_FILE_H

struct interrupt_frame;
void* int80h_command13_fopen(struct interrupt_frame *frame);
void* int80h_command14_fread(struct interrupt_frame *frame);
void* int80h_command15_fwrite(struct interrupt_frame *frame);
void* int80h_command16_fseek(struct interrupt_frame *frame);
void* int80h_command17_fstat(struct interrupt_frame *frame);
void* int80h_command18_fclose(struct interrupt_frame *frame);

#endif