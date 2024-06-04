#include <os/file.h>
#include <os/config.h>
#include <os/memory.h>
#include <os/kheap.h>
#include <os/status.h>
#include <os/fat16.h>
#include <os/string.h>
#include <os/disk.h>
#include <os/kernel.h>

static struct filesystem *filesystems[MAX_FILESYSTEMS];
static struct file_descriptor *file_descriptors[MAX_FILE_DESCRIPTORS];

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

static void fs_insert_filesystem(struct filesystem *filesystem)
{
    struct filesystem **fs = fs_get_free_filesystem();
    if (!fs)
    {
        kernel_panic("Problems inserting filesystem");
    }
    *fs = filesystem;
}

static void fs_load()
{
    memset(filesystems, 0, sizeof(filesystems));
    fs_insert_filesystem(fat16_init());
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
    return file_descriptors[fd - 1];
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

static FILE_MODE file_get_mode_by_string(const char *str)
{
    FILE_MODE mode = FILE_MODE_INVALID;
    if (!strncmp(str, "r", 1))
    {
        mode = FILE_MODE_READ;
    }
    else if (!strncmp(str, "w", 1))
    {
        mode = FILE_MODE_WRITE;
    }
    else if (!strncmp(str, "a", 1))
    {
        mode = FILE_MODE_APPEND;
    }
    return mode;
}

int fopen(const char *filename, const char *str)
{
    int res = 0;
    struct path_root *root = path_parser_parse(filename, NULL);
    if (!root)
    {
        res = -EINVARG;
        goto out;
    }
    if (!root->first)
    {
        res = -EINVARG;
        goto out;
    }

    struct disk *disk = disk_get(root->drive_no);
    if (!disk)
    {
        res = -EIO;
        goto out;
    }
    if (!disk->fs)
    {
        res = -EIO;
        goto out;
    }

    FILE_MODE mode = file_get_mode_by_string(str);
    if (mode == FILE_MODE_INVALID)
    {
        res = -EINVARG;
        goto out;
    }

    void *descriptor_private_data = disk->fs->open(disk, root->first, mode);
    if (ISERR(descriptor_private_data))
    {
        res = ERROR_I(descriptor_private_data);
        goto out;
    }

    struct file_descriptor *desc = NULL;
    res = file_new_descriptor(&desc);
    if (res < 0)
    {
        goto out;
    }
    desc->disk = disk;
    desc->fs = disk->fs;
    desc->private = descriptor_private_data;
    res = desc->index;

out:

    if (res < 0)
        res = 0;
    return res;
}

int fread(void *ptr, u32 size, int fd)
{
    if (size == 0 || fd < 1)
        return -EINVARG;

    struct file_descriptor *desc = file_get_descriptor(fd);
    if (!desc)
        return -EINVARG;

    return desc->fs->read(desc->disk, desc->private, size, (char *)ptr);
}

int fseek(int fd, u32 offset, FILE_SEEK_MODE mode)
{
    struct file_descriptor *desc = file_get_descriptor(fd);
    if (!desc)
        return -EIO;

    return desc->fs->seek(desc->private, offset, mode);
}

int fstat(int fd, struct file_stat *stat)
{
    struct file_descriptor *desc = file_get_descriptor(fd);
    if (!desc)
        return -EIO;

    return desc->fs->stat(desc->disk, desc->private, stat);
}

static void file_free_descriptor(struct file_descriptor *desc)
{
    file_descriptors[desc->index - 1] = NULL;
    kfree(desc);
}

int fclose(int fd)
{
    struct file_descriptor *desc = file_get_descriptor(fd);
    if (!desc)
        return -EIO;

    int res = desc->fs->close(desc->private);
    if (res == ALL_OK)
        file_free_descriptor(desc);

    return res;
}

void tree(int index)
{
    struct disk *disk = disk_get(index);
    if (!disk)
    {
        return;
    }
    if (!disk->fs)
    {
        return;
    }
    disk->fs->tree(disk);
}