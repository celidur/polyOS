#include <os/fat16.h>
#include <os/disk.h>
#include <os/string.h>
#include <os/status.h>
#include <os/streamer.h>
#include <os/kheap.h>
#include <os/types.h>
#include <os/memory.h>


#define FAT16_SIGNATURE 0x29
#define FAT16_FAT_ENTRY_SIZE 0x02
#define FAT16_BAD_SECTOR 0xFF7
#define FAT16_UNUSED 0x00

typedef unsigned int FAT_ITEM_TYPE;
#define FAT_ITEM_TYPE_DIRECTORY 0x00
#define FAT_ITEM_TYPE_FILE 0x01

// FAT directory entry attributes (bit flags)
#define FAT_FILE_READ_ONLY 0x01
#define FAT_FILE_HIDDEN 0x02
#define FAT_FILE_SYSTEM 0x04
#define FAT_FILE_VOLUME_LABEL 0x08
#define FAT_FILE_SUBDIRECTORY 0x10
#define FAT_FILE_ARCHIVE 0x20
#define FAT_FILE_DEVICE 0x40
#define FAT_FILE_RESERVED 0x80

struct fat_header_extended
{
    u8 drive_number;
    u8 win_nt_bit;
    u8 signature;
    u32 volume_id;
    u8 volume_label[11];
    u8 system_id[8];
} __attribute__((packed));

struct fat_header
{
    u8 jump[3];
    u8 oem[8];
    u16 bytes_per_sector;
    u8 sectors_per_cluster;
    u16 reserved_sectors;
    u8 fat_copies;
    u16 root_dir_entries;
    u16 number_of_sectors;
    u8 media_type;
    u16 sectors_per_fat;
    u16 sectors_per_track;
    u16 number_of_heads;
    u32 hidden_sectors;
    u32 sectors_big;
} __attribute__((packed));

struct fat_h
{
    struct fat_header primary_header;
    union fat_h_e
    {
        struct fat_header_extended extended_header;
    } shared;
};

struct fat_directory_item
{
    u8 filename[8];
    u8 ext[3];
    u8 attributes;
    u8 reserved;
    u8 creation_time_thenths_of_seconds;
    u16 creation_time;
    u16 creation_date;
    u16 last_access;
    u16 first_cluster_high;
    u16 last_modified_time;
    u16 last_modified_date;
    u16 first_cluster_low;
    u32 filesize;
} __attribute__((packed));

struct fat_directory
{
    struct fat_directory_item *items;
    int total;
    int sector_pos;
    int ending_sector_pos;
};

struct fat_item
{
    FAT_ITEM_TYPE type;
    union
    {
        struct fat_directory *directory;
        struct fat_directory_item *item;
    };
};

struct fat_file_descriptor
{
    struct fat_item *item;
    u32 pos;
};

struct fat_private
{
    struct fat_h header;
    struct fat_directory root_directory;

    struct disk_stream *cluser_read_stream;
    struct disk_stream *fat_read_stream;
    struct disk_stream *directory_stream;
};

int fat16_resolve(struct disk *disk);
void *fat16_open(struct disk *disk, struct path_part *path, FILE_MODE mode);
int fat16_read(struct disk *disk, void *descriptor, u32 size, u32 nmemb, void *out_ptr);
int fat16_seek(void *private, u32 offset, FILE_SEEK_MODE mode);
int fat16_stat(struct disk *disk, void *private, struct file_stat *stat);
int fat16_close(void *private);

struct filesystem fat16_fs = {
    .resolve = fat16_resolve,
    .open = fat16_open,
    .read = fat16_read,
    .seek = fat16_seek,
    .stat = fat16_stat,
    .close = fat16_close,
};

struct filesystem *fat16_init()
{
    strcpy(fat16_fs.name, "FAT16");
    return &fat16_fs;
}

static void fat16_init_private(struct disk *disk, struct fat_private *private)
{
    memset(private, 0, sizeof(struct fat_private));
    private->cluser_read_stream = disk_streamer_new(disk->id);
    private->fat_read_stream = disk_streamer_new(disk->id);
    private->directory_stream = disk_streamer_new(disk->id);
}

static int fat16_sector_to_absolute(struct disk *disk, int sector)
{
    return sector * disk->sector_size;
}

static int fat16_get_total_items_for_directory(struct disk *disk, u32 directory_start_sector)
{
    struct fat_directory_item item;
    struct fat_directory_item empty_item;
    memset(&empty_item, 0, sizeof(struct fat_directory_item));

    struct fat_private *private = disk->fs_private;

    int i = 0;
    int directory_start_pos = directory_start_sector * disk->sector_size;
    struct disk_stream *stream = private->directory_stream;
    if (disk_streamer_seek(stream, directory_start_pos) != ALL_OK)
        return -EIO;

    while (1)
    {
        if (disk_streamer_read(stream, &item, sizeof(item)) != ALL_OK)
            return -EIO;
        if (item.filename[0] == 0x00)
            break;
        if (item.filename[0] == 0xE5)
            continue;
        i++;
    };
    return i;
}

static int fat16_get_root_directory(struct disk *disk, struct fat_private *fat_private, struct fat_directory *directory)
{
    struct fat_header *header = &fat_private->header.primary_header;
    int root_directory_sector_pos = (header->fat_copies * header->sectors_per_fat) + header->reserved_sectors;
    int root_dir_entries = fat_private->header.primary_header.root_dir_entries;
    int root_dir_size = root_dir_entries * sizeof(struct fat_directory_item);
    int total_sectors = root_dir_size / disk->sector_size;
    if (root_dir_size % disk->sector_size)
        total_sectors++;

    int total_items = fat16_get_total_items_for_directory(disk, root_directory_sector_pos);
    struct fat_directory_item *dir = kzalloc(root_dir_size);
    if (!dir)
        return -ENOMEM;

    struct disk_stream *stream = fat_private->directory_stream;
    if (disk_streamer_seek(stream, fat16_sector_to_absolute(disk, root_directory_sector_pos)) != ALL_OK)
        return -EIO;

    if (disk_streamer_read(stream, dir, root_dir_size) != ALL_OK)
        return -EIO;

    directory->items = dir;
    directory->total = total_items;
    directory->sector_pos = root_directory_sector_pos;
    directory->ending_sector_pos = root_directory_sector_pos + (root_dir_size / disk->sector_size);
    return ALL_OK;
}

int fat16_resolve(struct disk *disk)
{
    int res = 0;
    struct fat_private *private = kzalloc(sizeof(struct fat_private));
    fat16_init_private(disk, private);

    disk->fs_private = private;
    disk->fs = &fat16_fs;

    struct disk_stream *stream = disk_streamer_new(disk->id);
    if (!stream)
    {
        res = -ENOMEM;
        goto out;
    }

    if (disk_streamer_read(stream, &private->header, sizeof(private->header)) != ALL_OK)
    {
        res = -EIO;
        goto out;
    }

    if (private->header.shared.extended_header.signature != FAT16_SIGNATURE)
    {
        res = -EFSNOTUS;
        goto out;
    }

    if (fat16_get_root_directory(disk, private, &private->root_directory) != ALL_OK)
    {
        res = -EIO;
        goto out;
    }

out:
    if (stream)
        disk_streamer_close(stream);

    if (res < 0)
    {
        kfree(private);
        disk->fs_private = NULL;
    }
    return res;
}

static void fat16_to_poper_string(char **out, const char *in)
{
    while (*in != 0x00 && *in != 0x20)
    {
        **out = *in;
        *out += 1;
        in += 1;
    }

    if (*in == 0x20)
        **out = 0x00;
}

static void fat16_get_full_relative_filename(struct fat_directory_item *item, char *out, int max_len)
{
    memset(out, 0, max_len);
    char *ptr = out;
    fat16_to_poper_string(&ptr, (const char *)item->filename);
    if (item->ext[0] != 0x00 && item->ext[0] != 0x20)
    {
        *ptr++ = '.';
        fat16_to_poper_string(&ptr, (const char *)item->ext);
    }
}

static struct fat_directory_item *fat16_clone_directory_item(struct fat_directory_item *item, int size)
{
    if (size < sizeof(struct fat_directory_item))
        return NULL;
    struct fat_directory_item *new_item = kzalloc(size);
    if (!new_item)
        return NULL;
    memcpy(new_item, item, size);
    return new_item;
}

// verify
static u32 fat32_get_first_cluster(struct fat_directory_item *item)
{
    return (item->first_cluster_high << 16) | item->first_cluster_low;
}

static int fat16_cluster_to_sector(struct fat_private *private, int cluser)
{
    return private->root_directory.ending_sector_pos + ((cluser - 2) * private->header.primary_header.sectors_per_cluster);
}

static u32 fat16_get_first_fat_sector(struct fat_private *private)
{
    return private->header.primary_header.reserved_sectors;
}

static int fat16_get_fat_entry(struct disk *disk, int cluster)
{
    struct fat_private *private = disk->fs_private;
    struct disk_stream *stream = private->fat_read_stream;
    if (!stream)
        return -EIO;

    u32 fat_table_pos = fat16_get_first_fat_sector(private) * disk->sector_size;
    if (disk_streamer_seek(stream, fat_table_pos * (cluster * FAT16_FAT_ENTRY_SIZE)) < 0)
        return -EIO;

    u16 result = 0;
    if (disk_streamer_read(stream, &result, sizeof(result)) < 0)
        return -EIO;
    return result;
}

static int fat16_get_cluster_for_offset(struct disk *disk, int starting_cluster, int offset)
{
    struct fat_private *private = disk->fs_private;
    int cluster_size = private->header.primary_header.sectors_per_cluster * disk->sector_size;
    int cluster = starting_cluster;
    int cluster_ahead = offset / cluster_size;
    for (int i = 0; i < cluster_ahead; i++)
    {
        int entry = fat16_get_fat_entry(disk, cluster);
        if (entry == 0xFF8 || entry == 0xFFF || entry == FAT16_BAD_SECTOR || entry == 0xFF0 || entry == 0xFF6 || entry == 0x00)
            return -EIO;
        cluster = entry;
    }
    return cluster;
}

static int fat16_read_internal_from_stream(struct disk *disk, struct disk_stream *stream, int cluster, int offset, void *buffer, int size)
{
    struct fat_private *private = disk->fs_private;
    int cluster_size = private->header.primary_header.sectors_per_cluster * disk->sector_size;
    int cluster_use = fat16_get_cluster_for_offset(disk, cluster, offset);
    if (cluster_use < 0)
        return cluster_use;

    int cluster_offset = offset % cluster_size;

    int starting_sector = fat16_cluster_to_sector(private, cluster_use);
    int starting_pos = (starting_sector * disk->sector_size) + cluster_offset;
    int total_to_read = size > cluster_size ? cluster_size : size;
    int res = disk_streamer_seek(stream, starting_pos);
    if (res != ALL_OK)
        return res;

    res = disk_streamer_read(stream, buffer, total_to_read);
    if (res != ALL_OK)
        return res;

    size -= total_to_read;
    if (size > 0)
        return fat16_read_internal_from_stream(disk, stream, cluster, offset + total_to_read, buffer + total_to_read, size);
    return ALL_OK;
}

static int fat16_read_internal(struct disk *disk, int starting_cluster, int offset, void *buffer, int size)
{
    struct fat_private *private = disk->fs_private;
    struct disk_stream *stream = private->cluser_read_stream;
    return fat16_read_internal_from_stream(disk, stream, starting_cluster, offset, buffer, size);
}

static void fat16_free_directory(struct fat_directory *directory)
{
    if (!directory)
        return;

    if (directory->items)
        kfree(directory->items);
    kfree(directory);
}

static void fat16_free_item(struct fat_item *item)
{
    if (!item)
        return;

    if (item->type == FAT_ITEM_TYPE_FILE)
    {
        kfree(item->item);
    }
    else if (item->type == FAT_ITEM_TYPE_DIRECTORY)
    {
        fat16_free_directory(item->directory);
    }
    kfree(item);
}

static struct fat_directory *fat16_load_fat_directory(struct disk *disk, struct fat_directory_item *item)
{
    int res = 0;
    struct fat_private *private = disk->fs_private;
    struct fat_directory *directory = NULL;
    if (!(item->attributes & FAT_FILE_SUBDIRECTORY))
    {
        res = -EINVARG;
        goto out;
    }

    directory = kzalloc(sizeof(struct fat_directory));
    if (!directory)
    {
        res = -ENOMEM;
        goto out;
    }

    int cluster = fat32_get_first_cluster(item);
    int cluster_sector = fat16_cluster_to_sector(private, cluster);
    int total_items = fat16_get_total_items_for_directory(disk, cluster_sector);
    directory->total = total_items;
    int directory_size = total_items * sizeof(struct fat_directory_item);
    directory->items = kzalloc(directory_size);
    if (!directory->items)
    {
        res = -ENOMEM;
        goto out;
    }

    res = fat16_read_internal(disk, cluster, 0x00, directory->items, directory_size);

out:
    if (res != ALL_OK)
    {
        fat16_free_directory(directory);
        directory = NULL;
    }
    return directory;
}

static struct fat_item *fat16_new_fat_item_for_directory_item(struct disk *disk, struct fat_directory_item *item)
{
    struct fat_item *f_item = kzalloc(sizeof(struct fat_item));
    if (!f_item)
        return NULL;

    if (item->attributes & FAT_FILE_SUBDIRECTORY)
    {
        f_item->directory = fat16_load_fat_directory(disk, item);
        f_item->type = FAT_ITEM_TYPE_DIRECTORY;
        return f_item;
    }

    f_item->type = FAT_ITEM_TYPE_FILE;
    f_item->item = fat16_clone_directory_item(item, sizeof(struct fat_directory_item));
    return f_item;
}

static struct fat_item *fat16_find_item_in_directory(struct disk *disk, struct fat_directory *directory, const char *name)
{
    char tmp_filename[MAX_PATH];
    for (int i = 0; i < directory->total; i++)
    {
        fat16_get_full_relative_filename(&directory->items[i], tmp_filename, sizeof(tmp_filename));
        if (istrncmp(tmp_filename, name, sizeof(tmp_filename)) == 0)
            return fat16_new_fat_item_for_directory_item(disk, &directory->items[i]);
    }
    return NULL;
}

static struct fat_item *fat16_get_directory_entry(struct disk *disk, struct path_part *path)
{
    struct fat_private *private = disk->fs_private;
    struct fat_item *root = fat16_find_item_in_directory(disk, &private->root_directory, path->part);
    if (!root)
        return NULL;

    struct fat_item *current_item = root;

    struct path_part *next_part = path->next;
    while (next_part != NULL)
    {
        if (current_item->type != FAT_ITEM_TYPE_DIRECTORY)
            return NULL;

        struct fat_item *tmp_item = fat16_find_item_in_directory(disk, current_item->directory, next_part->part);
        fat16_free_item(current_item);
        current_item = tmp_item;
        next_part = next_part->next;
    }
    return current_item;
}

void *fat16_open(struct disk *disk, struct path_part *path, FILE_MODE mode)
{
    if (mode != FILE_MODE_READ)
        return ERROR(-ERDONLY);

    struct fat_file_descriptor *descriptor = kzalloc(sizeof(struct fat_file_descriptor));
    if (!descriptor)
        return ERROR(-ENOMEM);

    descriptor->item = fat16_get_directory_entry(disk, path);
    if (!descriptor->item)
    {
        kfree(descriptor);
        return ERROR(-EBADPATH);
    }

    descriptor->pos = 0;
    return descriptor;
}

int fat16_read(struct disk *disk, void *descriptor, u32 size, u32 nmemb, void *out_ptr)
{
    struct fat_file_descriptor *desc = descriptor;
    struct fat_directory_item *item = desc->item->item;
    int offset = desc->pos;
    for (u32 i = 0; i < nmemb; i++)
    {
        int res = fat16_read_internal(disk, fat32_get_first_cluster(item), offset, out_ptr, size);
        if (res < 0)
            return res;
        offset += size;
        out_ptr += size;
    }

    return nmemb;
}

int fat16_seek(void *private, u32 offset, FILE_SEEK_MODE mode)
{
    struct fat_file_descriptor *desc = private;
    struct fat_item *item = desc->item;
    if (item->type != FAT_ITEM_TYPE_FILE)
        return -EINVARG;

    struct fat_directory_item *dir_item = item->item;
    if (offset > dir_item->filesize)
        return -EIO;

    switch (mode)
    {
    case FILE_SEEK_SET:
        desc->pos = offset;
        break;
    case FILE_SEEK_CUR:
        desc->pos += offset;
        break;
    case FILE_SEEK_END:
        desc->pos = dir_item->filesize - offset;
        break;
    default:
        return -EINVARG;
        break;
    }
    if (desc->pos > dir_item->filesize)
        return -EIO;
    return ALL_OK;
}

int fat16_stat(struct disk *disk, void *private, struct file_stat *stat)
{
    struct fat_file_descriptor *desc = private;
    struct fat_item *item = desc->item;
    if (item->type != FAT_ITEM_TYPE_FILE)
        return -EINVARG;

    struct fat_directory_item *dir_item = item->item;
    stat->size = dir_item->filesize;
    stat->flags = 0;

    if (dir_item->attributes & FAT_FILE_READ_ONLY)
        stat->flags |= FILE_STAT_READ_ONLY;
    return ALL_OK;
}

static void fat16_free_file_descriptor(struct fat_file_descriptor *desc)
{
    fat16_free_item(desc->item);
    kfree(desc);
}

int fat16_close(void *private)
{
    fat16_free_file_descriptor((struct fat_file_descriptor *)private);
    return ALL_OK;
}