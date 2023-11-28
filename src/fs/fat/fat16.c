#include "fat16.h"
#include "string/string.h"
#include "status.h"
#include "disk/disk.h"
#include "disk/streamer.h"
#include "memory/memory.h"
#include "memory/heap/kheap.h"
#include <stdint.h>
#include "kernel.h"
#include "config.h"

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
    uint8_t drive_number;
    uint8_t win_nt_bit;
    uint8_t signature;
    uint32_t volume_id;
    uint8_t volume_label[11];
    uint8_t system_id[8];
} __attribute__((packed));

struct fat_header
{
    uint8_t jump[3];
    uint8_t oem[8];
    uint16_t bytes_per_sector;
    uint8_t sectors_per_cluster;
    uint16_t reserved_sectors;
    uint8_t fat_copies;
    uint16_t root_dir_entries;
    uint16_t number_of_sectors;
    uint8_t media_type;
    uint16_t sectors_per_fat;
    uint16_t sectors_per_track;
    uint16_t number_of_heads;
    uint32_t hidden_sectors;
    uint32_t sectors_big;
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
    uint8_t filename[8];
    uint8_t ext[3];
    uint8_t attributes;
    uint8_t reserved;
    uint8_t creation_time_thenths_of_seconds;
    uint16_t creation_time;
    uint16_t creation_date;
    uint16_t last_access;
    uint16_t first_cluster_high;
    uint16_t last_modified_time;
    uint16_t last_modified_date;
    uint16_t first_cluster_low;
    uint32_t file_size;
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
    uint32_t pos;
};

struct fat_private
{
    struct fat_h header;
    struct fat_directory root_directory;

    struct disk_streamer *cluser_read_stream;
    struct disk_streamer *fat_read_stream;
    struct disk_streamer *directory_stream;
};

int fat16_resolve(struct disk *disk);

void *fat16_open(struct disk *disk, struct path_part *path, FILE_MODE mode);
struct filesystem fat16_fs = {
    .resolve = fat16_resolve,
    .open = fat16_open,
};

void fat16_to_poper_string(char **out, const char *in)
{
    int i = 0;
    const char space = ' ';
    while (!in[i] && in[i] != space)
    {
        *out[i] = in[i];
        i++;
    }
    if (in[i] == space)
    {
        *out[i] = 0;
    }
}

void fat16_get_full_relative_filename(struct fat_directory_item *item, char *out, int max_len)
{
    memset(out, 0, max_len);
    char *ptr = out;
    fat16_to_poper_string(&ptr, (const char *)item->filename);
    if (item->ext[0] != 0x20 && item->ext[0] != 0x00)
    {
        *ptr++ = '.';
        fat16_to_poper_string(&ptr, (const char *)item->ext);
    }
}

struct fat_directory_item *fat16_clone_directory_item(struct fat_directory_item *item, int size)
{
    if (size < sizeof(struct fat_directory_item))
    {
        return NULL;
    }
    struct fat_directory_item *new_item = kzalloc(size);
    if (!new_item)
    {
        return NULL;
    }
    memcpy(new_item, item, size);
    return new_item;
}

static uint32_t fat32_get_first_cluster(struct fat_directory_item *item)
{
    return (item->first_cluster_high << 16) | item->first_cluster_low;
}

static int fat16_cluster_to_sector(struct fat_private *private, int cluser)
{
    return private->root_directory.ending_sector_pos + ((cluser - 2) * private->header.primary_header.sectors_per_cluster);
}

static uint32_t fat16_get_first_fat_sector(struct fat_private *private)
{
    return private->header.primary_header.reserved_sectors;
}

static int fat16_get_fat_entry(struct disk *disk, int cluster)
{
    struct fat_private *private = disk->fs_private;
    struct disk_streamer *stream = private->fat_read_stream;
    if (!stream)
    {
        return -EIO;
    }

    uint32_t fat_table_pos = fat16_get_first_fat_sector(private) * disk->sector_size;
    if (diskstreamer_seek(stream, fat_table_pos * (cluster * FAT16_FAT_ENTRY_SIZE)) != ALL_OK)
    {
        return -EIO;
    }

    uint16_t result = 0;
    if (diskstreamer_read(stream, &result, sizeof(result)) < 0)
    {
        return -EIO;
    }
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
        if (entry == 0xFF8 || entry == 0xFFF)
        {
            return -EIO;
        }

        if (entry == FAT16_BAD_SECTOR)
        {
            return -EIO;
        }

        if (entry == 0xFF0 || entry == 0xFF6)
        {
            return -EIO;
        }

        if (entry == 0x00)
        {
            return -EIO;
        }
        cluster = entry;
    }
    return cluster;
}

static int fat16_read_internal_from_stream(struct disk *disk, struct disk_streamer *stream, int cluster, int offset, void *buffer, int size)
{
    struct fat_private *private = disk->fs_private;
    int cluster_size = private->header.primary_header.sectors_per_cluster * disk->sector_size;
    int cluster_use = fat16_get_cluster_for_offset(disk, cluster, offset);
    if (cluster_use < 0)
    {
        return cluster_use;
    }

    int cluster_offset = offset % cluster_size;

    int starting_sector = fat16_cluster_to_sector(private, cluster_use);
    int starting_pos = (starting_sector * disk->sector_size) * cluster_offset;
    int total_to_read = size > cluster_size ? cluster_size : size;
    int res = diskstreamer_seek(stream, starting_pos);
    if (res != ALL_OK)
    {
        return res;
    }

    res = diskstreamer_read(stream, buffer, total_to_read);
    if (res != ALL_OK)
    {
        return res;
    }

    size -= total_to_read;
    if (size > 0)
    {
        return fat16_read_internal_from_stream(disk, stream, cluster, offset + total_to_read, buffer + total_to_read, size);
    }
    return ALL_OK;
}

int fat16_sector_to_absolute(struct disk *disk, int sector)
{
    return sector * disk->sector_size;
}

int fat16_get_total_items_for_directory(struct disk *disk, uint32_t directory_start_sector)
{
    struct fat_directory_item item;
    struct fat_directory_item empty_item;
    memset(&empty_item, 0, sizeof(struct fat_directory_item));

    struct fat_private *private = disk->fs_private;

    int i = 0;
    int directory_start_pos = fat16_sector_to_absolute(disk, directory_start_sector);
    struct disk_streamer *stream = private->directory_stream;
    if (diskstreamer_seek(stream, directory_start_pos) != ALL_OK)
    {
        return -EIO;
    }

    while (1)
    {
        if (diskstreamer_read(stream, &item, sizeof(item)) != ALL_OK)
        {
            return -EIO;
        }
        if (item.filename[0] == 0x00)
        {
            break;
        }
        if (item.filename[0] == 0xE5)
        {
            continue;
        }
        i++;
    };
    return i;
}

static int fat16_read_internal(struct disk *disk, int starting_cluster, int offset, void *buffer, int size)
{
    struct fat_private *private = disk->fs_private;
    struct disk_streamer *stream = private->cluser_read_stream;
    return fat16_read_internal_from_stream(disk, stream, starting_cluster, offset, buffer, size);
}

void fat16_free_directory(struct fat_directory *directory)
{
    if (!directory)
    {
        return;
    }

    if (directory->items)
    {
        kfree(directory->items);
    }
    kfree(directory);
}

struct fat_directory *fat16_load_fat_directory(struct disk *disk, struct fat_directory_item *item)
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

    res = fat16_read_internal(disk, cluster, 0, directory->items, directory_size);
    if (res != ALL_OK)
    {
        goto out;
    }

out:
    if (res < 0)
    {
        fat16_free_directory(directory);
        directory = NULL;
    }
    return directory;
}

struct fat_item *fat16_new_fat_item_for_directory_item(struct disk *disk, struct fat_directory_item *item)
{
    struct fat_item *f_item = kzalloc(sizeof(struct fat_item));
    if (!f_item)
    {
        return NULL;
    }

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

struct fat_item *fat16_find_item_in_directory(struct disk *disk, struct fat_directory *directory, const char *name)
{
    char tmp_filename[MAX_PATH];
    for (int i = 0; i < directory->total; i++)
    {
        fat16_get_full_relative_filename(&directory->items[i], tmp_filename, MAX_PATH);
        if (!istrncmp(tmp_filename, name, MAX_PATH))
        {
            return fat16_new_fat_item_for_directory_item(disk, &directory->items[i]);
        }
    }
    return NULL;
}

void fat16_free_item(struct fat_item *item)
{
    if (!item)
    {
        return;
    }

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

struct fat_item *fat16_get_directory_entry(struct disk *disk, struct path_part *path)
{
    struct fat_private *private = disk->fs_private;
    struct fat_item *root = fat16_find_item_in_directory(disk, &private->root_directory, path->part);
    if (!root)
    {
        return NULL;
    }
    struct fat_item *current_item = NULL;

    struct path_part *next_part = path->next;
    current_item = root;
    while (next_part != NULL)
    {
        if (current_item->type != FAT_ITEM_TYPE_DIRECTORY)
        {
            return NULL;
        }

        struct fat_item *tmp_item = fat16_find_item_in_directory(disk, current_item->directory, next_part->part);
        fat16_free_item(current_item);
        current_item = tmp_item;
        next_part = next_part->next;
    }
    return current_item;
}

static void fat16_init_private(struct disk *disk, struct fat_private *private)
{
    memset(private, 0, sizeof(struct fat_private));
    private->cluser_read_stream = diskstreamer_new(disk->id);
    private->fat_read_stream = diskstreamer_new(disk->id);
    private->directory_stream = diskstreamer_new(disk->id);
}

int fat16_get_root_directory(struct disk *disk, struct fat_private *fat_private, struct fat_directory *directory)
{
    struct fat_header *header = &fat_private->header.primary_header;
    int root_directory_sector_pos = (header->fat_copies * header->sectors_per_fat) + header->reserved_sectors;
    int root_dir_entries = fat_private->header.primary_header.root_dir_entries;
    int root_dir_size = root_dir_entries * sizeof(struct fat_directory_item);
    int total_sectors = root_dir_size / disk->sector_size;
    if (root_dir_size % disk->sector_size)
    {
        total_sectors++;
    }

    int total_items = fat16_get_total_items_for_directory(disk, root_directory_sector_pos);
    struct fat_directory_item *dir = kzalloc(root_dir_size);
    if (!dir)
    {
        return -ENOMEM;
    }

    struct disk_streamer *stream = fat_private->directory_stream;
    if (diskstreamer_seek(stream, fat16_sector_to_absolute(disk, root_directory_sector_pos)) != ALL_OK)
    {
        return -EIO;
    }

    if (diskstreamer_read(stream, dir, root_dir_size) != ALL_OK)
    {
        return -EIO;
    }

    directory->items = dir;
    directory->total = total_items;
    directory->sector_pos = root_directory_sector_pos;
    directory->ending_sector_pos = root_directory_sector_pos + (root_dir_size / disk->sector_size);
    return ALL_OK;
}

struct filesystem *fat16_init()
{
    strcpy(fat16_fs.name, "FAT16");
    return &fat16_fs;
}

// By always returning 0, we say we can read any disk regardless of its filesystem.
int fat16_resolve(struct disk *disk)
{
    int res = 0;
    struct fat_private *private = kzalloc(sizeof(struct fat_private));
    fat16_init_private(disk, private);

    disk->fs_private = private;
    disk->fs = &fat16_fs;

    struct disk_streamer *stream = diskstreamer_new(disk->id);
    if (!stream)
    {
        res = -ENOMEM;
        goto out;
    }

    if (diskstreamer_read(stream, &private->header, sizeof(struct fat_h)) != ALL_OK)
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

    if (stream)
    {
        diskstreamer_close(stream);
    }
out:
    if (res < 0)
    {
        kfree(private);
        disk->fs_private = NULL;
    }
    return res;
}

void *fat16_open(struct disk *disk, struct path_part *path, FILE_MODE mode)
{
    if (mode != FILE_MODE_READ)
    {
        return ERROR(-ERDONLY);
    }
    struct fat_file_descriptor *descriptor = kzalloc(sizeof(struct fat_file_descriptor));
    if (!descriptor)
    {
        return ERROR(-ENOMEM);
    }

    descriptor->item = fat16_get_directory_entry(disk, path);
    if (!descriptor->item)
    {
        kfree(descriptor);
        return ERROR(-EBADPATH);
    }

    print("ijfijrgi\n");
    descriptor->pos = 0;
    return descriptor;
}