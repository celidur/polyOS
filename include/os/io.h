#ifndef IO_H
#define IO_H

#include <os/types.h>

u8 inb(u16 port);
u16 inw(u16 port);
u32 inl(u16 port);

void outb(u16 port, u8 data);
void outw(u16 port, u16 data);
void outl(u16 port, u32 data);

#endif