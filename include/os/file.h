#ifndef FILE_H
#define FILE_H

#include <os/pparser.h>
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
    FILE_STAT_FLAGS flags;
};

struct disk;

typedef void *(*FS_OPEN_FUNCTION)(void *fs_private, struct path_part *path, FILE_MODE mode);
typedef int (*FS_RESOLVE_FUNCTION)(struct disk *disk);
typedef int (*FS_READ_FUNCTION)(void *fs_private, void *fd_private, uint32_t size, void *out);
typedef int (*FS_SEEK_FUNCTION)(void *fd_private, uint32_t offset, FILE_SEEK_MODE mode);
typedef int (*FS_STAT_FUNCTION)(void *fd_private, struct file_stat *stat);
typedef int (*FS_CLOSE_FUNCTION)(void *fd_private);
typedef void (*FS_TREE_FUNCTION)(void *fs_private);
typedef int (*FS_WRITE_FUNCTION)(void *fs_private, void *fd_private, uint32_t size, void *in);

struct filesystem
{
    FS_OPEN_FUNCTION open;
    FS_RESOLVE_FUNCTION resolve;
    FS_READ_FUNCTION read;
    FS_SEEK_FUNCTION seek;
    FS_WRITE_FUNCTION write;

    FS_STAT_FUNCTION stat;
    FS_CLOSE_FUNCTION close;

    FS_TREE_FUNCTION tree;

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
int fopen(const char *filename, const char *str);
int fread(int fd, void *ptr, uint32_t size);
int fseek(int fd, uint32_t offset, FILE_SEEK_MODE mode);
int fstat(int fd, struct file_stat *stat);
int fwrite(int fd, void *ptr, u32 size);
int fclose(int fd);
void tree(int index);
struct filesystem *fs_resolve(struct disk *disk);

/*
FILE: fat16.c
*/
struct filesystem *fat16_init();

#endif