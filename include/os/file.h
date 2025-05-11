#ifndef FILE_H
#define FILE_H

#include <os/types.h>

typedef unsigned int FILE_SEEK_MODE;
enum
{
    FILE_SEEK_SET,
    FILE_SEEK_CUR,
    FILE_SEEK_END
};

typedef unsigned int FILE_MODE;
enum
{
    FILE_MODE_READ,
    FILE_MODE_WRITE,
    FILE_MODE_APPEND,
    FILE_MODE_INVALID
};

enum
{
    FILE_STAT_READ_ONLY = 0b00000001,
};

typedef unsigned int FILE_STAT_FLAGS;

struct file_stat
{
    uint32_t size;
};

int fopen(const char *filename, const char *str);
int fread(int fd, void *ptr, uint32_t size);
int fseek(int fd, uint32_t offset, FILE_SEEK_MODE mode);
int fstat(int fd, struct file_stat *stat);
int fwrite(int fd, void *ptr, u32 size);
int fclose(int fd);

#endif