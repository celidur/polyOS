#ifndef FILE_H
#define FILE_H

#include "pparser.h"

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

struct disk;

typedef void *(*FS_OPEN_FUNCTION)(struct disk *disk, struct path_part *path, FILE_MODE mode);
typedef int (*FS_RESOLVE_FUNCTION)(struct disk *disk);

struct filesystem
{
    FS_OPEN_FUNCTION open;
    FS_RESOLVE_FUNCTION resolve;
    char name[20];
};

struct file_descriptor
{
    int index;
    struct filesystem *fs;
    void *private;
    struct disk *disk;
};

void fs_init();
int fopen(const char *filename, FILE_MODE mode);
void fs_insert_filesytem(struct filesystem *fs);
struct filesystem *fs_resolve(struct disk *disk);

#endif