#include <os/terminal.h>
#include <os/string.h>
#include <os/types.h>
#include <os/io.h>
#include <os/vga.h>
#include <os/types.h>
#include <stdarg.h>
#include <os/memory.h>

#define MAX_BUFFER 1024

#define VGA_CTRL_REGISTER 0x3d4
#define VGA_DATA_REGISTER 0x3d5
#define VGA_OFFSET_LOW 0x0f
#define VGA_OFFSET_HIGH 0x0e

static uint16_t row_position = 0;
static uint16_t column_position = 0;

static color_t current_color = 0;

static uint16_t* buffer = 0;

int serial_write(const char *buf);

static bool ascii_is_printable(uint8_t c)
{
    return (c >= 0x20 && c <= 0x7E) || c == '\n';
}

void set_color(color_t background, color_t foreground){
    current_color = background << 4 | foreground;
}

void disable_cursor()
{
	outb(VGA_CTRL_REGISTER, 0x0A);
	outb(VGA_DATA_REGISTER, 0x20);
}

static void set_cursor(int offset) {
    outb(VGA_CTRL_REGISTER, VGA_OFFSET_HIGH);
    outb(VGA_DATA_REGISTER, (unsigned char) ((offset >> 8) & 0xff));
    outb(VGA_CTRL_REGISTER, VGA_OFFSET_LOW);
    outb(VGA_DATA_REGISTER, (unsigned char) (offset & 0xff));
}

static uint16_t terminal_make_char(uint8_t c, color_t color)
{
    return (color << 8) | c;
}

static void clear_row(uint16_t row)
{
    uint16_t blank = terminal_make_char(' ', current_color);
    for (int x = 0; x < get_screen_width(); x++)
    {
        buffer[row * get_screen_width() + x] = blank;
    }
}

void clear_screen()
{
    row_position = 0;
    column_position = 0;
    set_cursor(0);
    for (int y = 0; y < get_screen_height(); y++)
    {
        clear_row(y);
    }
}

void terminal_initialize()
{
    buffer = (uint16_t *)0xB8000;
    set_color(BLACK, WHITE);
    clear_screen();
}

static void new_line(){
    if (row_position < get_screen_height() - 1)
    {
        row_position++;
        column_position = 0;
        return;
    }
    row_position = get_screen_height() - 1;
    for (size_t row = 1; row < get_screen_height(); row++)
    {
        for (size_t col = 0; col < get_screen_width(); col++)
        {
            buffer[(row - 1) * get_screen_width() + col] = buffer[row * get_screen_width() + col];
        }
    }
    clear_row(get_screen_height() - 1);
    column_position = 0;
}

static void write_byte(uint8_t byte, uint8_t color)
{
    if (byte == '\n')
    {
        new_line();
        return;
    }
    if (column_position >= get_screen_width())
    {
        new_line();
    }
    buffer[row_position * get_screen_width() + column_position] = terminal_make_char(byte, color);
    column_position++;
    set_cursor(row_position * get_screen_width() + column_position);
}


void terminal_backspace()
{
    // TODO: rewrite this
    if (column_position > 0)
    {
        column_position--;
        buffer[row_position * get_screen_width() + column_position] = terminal_make_char(' ', current_color);
    } else if (row_position > 0){
        row_position--;
        column_position = get_screen_width() - 1;
        buffer[row_position * get_screen_width() + column_position] = terminal_make_char(' ', current_color);
    }
    set_cursor(row_position * get_screen_width() + column_position);
}

void terminal_writechar(uint8_t c, color_t color)
{
    if (c == '\n')
    {
        new_line();
        return;
    }
    if (c == '\b')
    {
        terminal_backspace();
        return;
    }
    write_byte(c, color);
}


void print_c(const char *str, color_t color)
{
    size_t len = strlen(str);
    for (size_t i = 0; i < len; i++)
    {
        terminal_writechar(str[i], color);
    }
}

void print(const char *str)
{
    print_c(str, current_color);
    // serial_write(str);
}

static char* itoa(s64 i) {
    static char str[22];
    memset(str, '0', 22);
    int loc = 21;
    str[loc] = '\0';
    char neg = 1;
    if (i >= 0){
        neg = 0;
        i = -i;
    }

    while (i){
        str[--loc] = '0' - (i % 10);
        i /= 10;
    }

    if (loc == 21){
        str[--loc] = '0';
    }
    if (neg){
        str[--loc] = '-';
    }
    return &str[loc];
}

static char* hex(uint32_t i){
    static char str[12];
    int loc = 11;
    str[loc] = '\0';
    while (i){
        int rem = i % 16;
        if (rem < 10){
            str[--loc] = '0' + rem;
        } else {
            str[--loc] = 'A' + (rem - 10);
        }
        i /= 16;
    }

    if (loc == 11){
        str[--loc] = '0';
    }
    
    return &str[loc];

}

int printf(const char *fmt, ...){
    va_list ap;
    const char* p;
    char* sval;
    int ival;
    char buff[MAX_BUFFER + 1];
    int i=0;
    
    va_start(ap, fmt);
    for (p = fmt; *p; p++){
        if (i >= MAX_BUFFER){
            buff[i] = '\0';
            print(buff);
            serial_write(buff);
            i = 0;
        }
        if (*p != '%'){
            buff[i++] = *p;
            continue;
        }
        switch (*++p){
            case 'd':
                ival = va_arg(ap, int);
                sval = itoa(ival);
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER){
                        buff[i] = '\0';
                        print(buff);
                        serial_write(buff);
                        i = 0;
                    }
                    buff[i++] = sval[j];
                }
                break;
            case 's':
                sval = va_arg(ap, char*);
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER){
                        buff[i] = '\0';
                        print(buff);
                        serial_write(buff);
                        i = 0;
                    }
                    buff[i++] = sval[j];
                }
                break;
            case 'c':
                ival = va_arg(ap, int);
                buff[i++] = ival;
                break;
            case 'x':
                sval = hex(va_arg(ap, uint32_t));
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER){
                        buff[i] = '\0';
                        print(buff);
                        serial_write(buff);
                        i = 0;
                    }
                    buff[i++] = sval[j];
                }
                break;
            default:
                buff[i++] = *p;
                break;
        }
    }

    va_end(ap);

    buff[i] = '\0';
    print(buff);
    serial_write(buff);

    return 0;
}

int serial_printf(const char *fmt, ...){
    va_list ap;
    const char* p;
    char* sval;
    u32 ival;
    u64 lval;
    char buff[MAX_BUFFER + 1];
    int i=0;

    va_start(ap, fmt);
    for (p = fmt; *p; p++){
        if (i >= MAX_BUFFER){
            buff[i] = '\0';
            serial_write(buff);
            i = 0;
        }
        if (*p != '%'){
            buff[i++] = *p;
            continue;
        }
        switch (*++p){
            case 'd':
                ival = va_arg(ap, int);
                sval = itoa(ival);
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER){
                        buff[i] = '\0';
                        serial_write(buff);
                        i = 0;
                    }
                    buff[i++] = sval[j];
                }
                break;
            case 'l':
                lval = va_arg(ap, u64);
                sval = itoa(lval);
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER){
                        buff[i] = '\0';
                        serial_write(buff);
                        i = 0;
                    }
                    buff[i++] = sval[j];
                }
                break;
            case 's':
                sval = va_arg(ap, char*);
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER){
                        buff[i] = '\0';
                        serial_write(buff);
                        i = 0;
                    }
                    buff[i++] = sval[j];
                }
                break;
            case 'c':
                ival = va_arg(ap, int);
                buff[i++] = ival;
                break;
            case 'x':
                sval = hex(va_arg(ap, uint32_t));
                for (int j = 0; sval[j] != '\0'; j++){
                    if (i >= MAX_BUFFER){
                        buff[i] = '\0';
                        serial_write(buff);
                        i = 0;
                    }
                    buff[i++] = sval[j];
                }
                break;
            default:
                buff[i++] = *p;
                break;
        }
    }

    va_end(ap);

    buff[i] = '\0';
    return serial_write(buff);
}