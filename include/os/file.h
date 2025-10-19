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

#endif