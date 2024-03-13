#ifndef SERIAL_H
#define SERIAL_H

#include <os/types.h>

#define SERIAL_COM1_BASE 0x3F8              /* COM1 base port */


enum BaudRate { Baud_115200 = 1, Baud_57600, Baud_19200, Baud_9600 };

void serial_configure(u16 port, u16 baudRate);

int serial_write(const char *buf);

#endif