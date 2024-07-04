#ifndef POLYOS_STDIO_H
#define POLYOS_STDIO_H

typedef unsigned int FILE_SEEK_MODE;
enum
{
    FILE_SEEK_SET,
    FILE_SEEK_CUR,
    FILE_SEEK_END
};

typedef unsigned int FILE_STAT_FLAGS;
enum
{
    FILE_STAT_READ_ONLY = 0b00000001,
};

struct file_stat
{
    int size;
    FILE_STAT_FLAGS flags;
};


int putchar(int c);
int printf(const char *fmt, ...);
int serial_printf(const char *fmt, ...);

int fopen(const char *filename, const char *mode);
int fread(int fd, void *ptr, int size);
int fwrite(int fd, void *ptr, int size);
int fseek(int fd, int offset, FILE_SEEK_MODE mode);
int fstat(int fd, struct file_stat *stat);
int fclose(int fd);

#endif