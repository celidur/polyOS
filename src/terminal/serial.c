#include <os/serial.h>
#include <os/io.h>
#include <os/string.h>
#include <os/types.h>


#define SERIAL_DATA_PORT(base) (base)
#define SERIAL_FIFO_COMMAND_PORT(base) (base + 2)
#define SERIAL_LINE_COMMAND_PORT(base) (base + 3)
#define SERIAL_MODEM_COMMAND_PORT(base) (base + 4)
#define SERIAL_LINE_STATUS_PORT(base) (base + 5)

#define SERIAL_LINE_ENABLE_DLAB 0x80

static unsigned short serial_port = 0x3F8; /* COM1 */


void serial_configure_baud_rate(unsigned short com, unsigned short divisor)
{
	outb(SERIAL_LINE_COMMAND_PORT(com), SERIAL_LINE_ENABLE_DLAB);
	outb(SERIAL_DATA_PORT(com), (divisor >> 8) & 0x00FF);
	outb(SERIAL_DATA_PORT(com), divisor & 0x00FF);
}

void serial_configure_line(unsigned short com)
{
    /* Bit: | 7 | 6 | 5 4 3 | 2 | 1 0 |
    * Content: | d | b | prty | s | dl |
    * Value: | 0 | 0 | 0 0 0 | 0 | 1 1 | = 0x03
    */
    outb(SERIAL_LINE_COMMAND_PORT(com), 0x03);
}
   
    
void serial_configure_fifo_buffer(unsigned short com) 
{
	outb(SERIAL_FIFO_COMMAND_PORT(com), 0xC7);
}


void serial_configure_modem(unsigned short com) 
{
	outb(SERIAL_MODEM_COMMAND_PORT(com), 0x03);
}
    
int serial_is_transmit_fifo_empty()
{
	/* 0x20 = 0010 0000 */
    return inb(SERIAL_LINE_STATUS_PORT(serial_port)) & 0x20;
}
    
    
void serial_write_byte(char byteData) 
{
	outb(serial_port, byteData);
}  
    
    
void serial_configure(unsigned short port, unsigned short baudRate) 
{
    serial_port = port;
	serial_configure_baud_rate(port, baudRate);
	serial_configure_line(port);
	serial_configure_fifo_buffer(port);
	serial_configure_modem(port);
}
    
    
int serial_write(const char *buf) 
{
    size_t len = strlen(buf);
	unsigned int bufferIndex = 0;
	while (bufferIndex < len) {
		if (serial_is_transmit_fifo_empty()) {
			serial_write_byte(buf[bufferIndex]);
			bufferIndex++;
		}
	}
	return 0;
}
    