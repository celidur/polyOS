#include "file.h"
#include "config.h"
#include "memory/memory.h"
#include "memory/heap/kheap.h"
#include "status.h"
#include "kernel.h"

struct filesystem *filesystems[MAX_FILESYSTEMS];
struct file_descriptor *file_descriptors[MAX_FILE_DESCRIPTORS];

static struct filesystem **fs_get_free_filesystem()
{
    for (int i = 0; i < MAX_FILESYSTEMS; i++)
    {
        if (filesystems[i] == NULL)
        {
            return &filesystems[i];
        }
    }
    return NULL;
}

void fs_insert_filesystem(struct filesystem *filesystem)
{
    struct filesystem **fs = fs_get_free_filesystem();
    if (!fs)
    {
        kernel_panic("Problems inserting filesystem");
    }
    *fs = filesystem;
}

static void fs_static_load()
{
    // load filesystems
}

void fs_load()
{
    memset(filesystems, 0, sizeof(filesystems));
    fs_static_load();
}

void fs_init()
{
    memset(file_descriptors, 0, sizeof(file_descriptors));
    fs_load();
}

static int file_new_descriptor(struct file_descriptor **desc_out)
{
    int res = -ENOMEM;
    for (int i = 0; i < MAX_FILE_DESCRIPTORS; i++)
    {
        if (!file_descriptors[i])
        {
            struct file_descriptor *desc = kzalloc(sizeof(struct file_descriptor));
            desc->index = i + 1;
            file_descriptors[i] = desc;
            *desc_out = desc;
            return 0;
        }
    }
    return res;
}

static struct file_descriptor *file_get_descriptor(int fd)
{
    if (fd <= 0 || fd >= MAX_FILE_DESCRIPTORS)
    {
        return NULL;
    }
    int index = fd - 1;
    return file_descriptors[index];
}

struct filesystem *fs_resolve(struct disk *disk)
{

    for (int i = 0; i < MAX_FILESYSTEMS; i++)
    {
        if (filesystems[i] && !filesystems[i]->resolve(disk))
        {
            return filesystems[i];
        }
    }
    return NULL;
}

int fopen(const char *filename, FILE_MODE mode)
{
    return -EIO;
}
