#include <os/fat16.h>
#include <os/disk.h>
#include <os/string.h>
#include <os/status.h>
#include <os/streamer.h>
#include <os/kheap.h>
#include <os/types.h>
#include <os/memory.h>
#include <os/config.h>

// check https://github.com/eerimoq/simba/blob/master/src/filesystems/fat16.c

#include <os/terminal.h>
#include <os/string.h>

#define FAT16_SIGNATURE 0x29
#define FAT16_FAT_ENTRY_SIZE 0x02
#define FAT16_BAD_SECTOR 0xFFF7
#define FAT16_UNUSED 0x00
#define FAT16_ENTRY_FREE 0xE5
#define FAT16_EOC 0xFFFF
#define FAT16_MIN_EOC 0xFFF8

typedef unsigned int FAT_ITEM_TYPE;
#define FAT_ITEM_UNKNOWN 0x00
#define FAT_ITEM_TYPE_ROOT_DIRECTORY 0x01
#define FAT_ITEM_TYPE_SUBDIRECTORY 0x02
#define FAT_ITEM_TYPE_FILE 0x04

#define FAT_DIR_ATTR_LONG_NAME_MASK 0x0f
#define FAT_DIR_ATTR_LONG_NAME 0x0f

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

struct fat_entry_alias
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

struct raw_data
{
    union
    {
        struct fat_entry_alias alias;
        u8 data[32];
    }; 
};

struct fat_entry_t
{
    struct fat_entry_alias alias;
    char filename[256];
    u32 entry_offset;
}

struct fat_item;

struct fat_dir_item_t
{
    struct fat_item item;
    struct fat_dir_item_t *next;
    struct fat_dir_item_t *prev;
};

struct fat_dir_t
{
    struct fat_entry_t self;
    struct fat_dir_item_t *items;
}

struct fat_root_dir_t
{
    struct fat_dir_item_t *items;
    int sector_pos;
    int ending_sector_pos;
};

struct fat_item
{
    FAT_ITEM_TYPE type;
    union
    {
        struct fat_root_dir_t *root_dir;
        struct fat_dir_t *directory;
        struct fat_entry_t *item;
    };
};

struct fat_file_descriptor
{
    struct fat_item *item;
    FILE_MODE mode;
    u32 pos;
};

struct fat_private
{
    struct fat_h header;
    struct fat_item root_directory;

    struct disk_stream *cluser_read_stream;
    struct disk_stream *fat_read_stream;
    struct disk_stream *fat_write_stream;
    struct disk_stream *directory_stream;
};

int fat16_resolve(struct disk *disk);
void *fat16_open(struct disk *disk, struct path_part *path, FILE_MODE mode);
int fat16_read(struct disk *disk, void *descriptor, u32 size, void *out_ptr);
int fat16_seek(void *private, u32 offset, FILE_SEEK_MODE mode);
int fat16_stat(struct disk *disk, void *private, struct file_stat *stat);
int fat16_close(void *private);
void tree_fat16(struct disk *disk);

struct filesystem fat16_fs = {
    .resolve = fat16_resolve,
    .open = fat16_open,
    .read = fat16_read,
    .seek = fat16_seek,
    .stat = fat16_stat,
    .close = fat16_close,
    .tree = tree_fat16,
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
    private->fat_write_stream = disk_streamer_new(disk->id);
}

static inline bool dir_is_long_name(const struct fat_dir_t *item)
{
    return (item->attributes & FAT_DIR_ATTR_LONG_NAME_MASK) == FAT_DIR_ATTR_LONG_NAME;
}

static inline int fat16_sector_to_absolute(struct disk *disk, int sector)
{
    return sector * disk->sector_size;
}

static int fat16_make_name_from_83(fat_entry_alias* alias, char* buff) {
    if (!alias || !name)
        return -EINVARG
    u8 i, pos;

    pos = 0;

    for (i = 0; i < 11; i++){
        if (alias->filename[i] == 0x20)
            continue;
        
        if (i == 8)
            buff[pos++] = '.';

        buff[pos++] = alias->filename[i];
    }
    buff[pos] = 0x00;
    return 0;
}

static int fat16_read_entry(struct raw_data *data, struct fat_entry_t *entry) {
    if (!entry || !data || !data->alias.filename[0])
        return 0;

    char* long_name = entry->filename;
    if (dir_is_long_name(data->alias)){
        uint16_t char_offset = ((data->data[0] & 0x3f) - 1) * 13;

        if (char_offset + 12 < sizeof(entry->filename)){
            // for now we assume that is pure ascii
            long_name[char_offset + 0] = data->data[1];
            long_name[char_offset + 1] = data->data[3];
            long_name[char_offset + 2] = data->data[5];
            long_name[char_offset + 3] = data->data[7];
            long_name[char_offset + 4] = data->data[9];
            long_name[char_offset + 5] = data->data[14];
            long_name[char_offset + 6] = data->data[16];
            long_name[char_offset + 7] = data->data[18];
            long_name[char_offset + 8] = data->data[20];
            long_name[char_offset + 9] = data->data[22];
            long_name[char_offset + 10] = data->data[24];
            long_name[char_offset + 11] = data->data[28];
            long_name[char_offset + 12] = data->data[30];
        }
        return 1;
    } 
    if (long_name[0] == 0x00){
        // copy short name if long name is not present
        fat16_make_name_from_83(&data->alias, long_name);
    }
    memcpy(&entry->alias, &data->alias, sizeof(fat_entry_alias));
    return 2;
}

// TODO: Clean memory
static int fat16_get_root_directory(struct disk *disk, struct fat_private *fat_private, struct fat_item *root)
{
    struct fat_root_dir_t *directory = kzalloc(sizeof(struct fat_root_dir_t));
    if (!directory)
        return -ENOMEM;

    root->type = FAT_ITEM_TYPE_ROOT_DIRECTORY;
    root->root_dir = directory;
    struct fat_header *header = &fat_private->header.primary_header;
    int root_directory_sector_pos = (header->fat_copies * header->sectors_per_fat) + header->reserved_sectors;
    int root_dir_entries = fat_private->header.primary_header.root_dir_entries;
    int root_dir_size = root_dir_entries * sizeof(struct raw_data);
    int total_sectors = root_dir_size / disk->sector_size;
    if (root_dir_size % disk->sector_size)
        total_sectors++;

    struct raw_data *data = kzalloc(root_dir_size);
    if (!data)
        return -ENOMEM;

    struct disk_stream *stream = fat_private->directory_stream;
    if (disk_streamer_seek(stream, fat16_sector_to_absolute(disk, root_directory_sector_pos)) != ALL_OK)
        return -EIO;

    if (disk_streamer_read(stream, data, root_dir_size) != ALL_OK)
        return -EIO;

    struct fat_dir_item_t *start = NULL;
    struct fat_dir_item_t *current = NULL;
    struct fat_entry_t *entry = NULL;
    int i = 0;
    while (i < root_dir_entries)
    {
        struct fat_dir_t *dir = data[i].alias;
        if (dir->filename[0] == 0x00)
            break;
        if (dir->filename[0] == 0xE5)
        {
            i++;
            continue;
        }
        if (!entry){
            entry = kzalloc(sizeof(struct fat_entry_t));
            if (!entry)
                return -ENOMEM;
            entry->entry_offset = i * sizeof(struct raw_data) + fat16_sector_to_absolute(disk, root_directory_sector_pos);
        }
        int res = fat16_read_entry(&data[i], entry);
        if (res == 2){
            if (!start){
                start = kzalloc(sizeof(struct fat_dir_item_t));
                current = start;
            } else {
                current->next = kzalloc(sizeof(struct fat_dir_item_t));
                current->next->prev = current;
                current = current->next;
            }
            struct 

            current->item = entry;


            entry = NULL;
        }
        i++;
    }
    if (entry)
        kfree(entry);

    directory->items = dir;
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

static void fat16_get_full_relative_filename(struct fat_dir_t *item, char *out, int max_len)
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

static struct fat_dir_t *fat16_clone_directory_item(struct fat_dir_t *item, int size)
{
    if (size < sizeof(struct fat_dir_t))
        return NULL;
    struct fat_dir_t *new_item = kzalloc(size);
    if (!new_item)
        return NULL;
    memcpy(new_item, item, size);
    return new_item;
}

// verify
static u32 fat32_get_first_cluster(struct fat_dir_t *item)
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

static int fat16_release_cluster(struct disk *disk, int cluster)
{
    struct fat_private *private = disk->fs_private;
    struct disk_stream *stream = private->fat_write_stream;
    if (!stream)
        return -EIO;

    u32 fat_table_pos = fat16_get_first_fat_sector(private) * disk->sector_size;
    if (disk_streamer_seek(stream, fat_table_pos * (cluster * FAT16_FAT_ENTRY_SIZE)) < 0)
        return -EIO;

    u16 result = 0;
    if (disk_streamer_write(stream, &result, sizeof(result)) < 0)
        return -EIO;
    return ALL_OK;
}

static int fat16_release_chain(struct disk *disk, int cluster)
{
    int next_cluster = fat16_get_fat_entry(disk, cluster);
    if (next_cluster < 0)
        return next_cluster;

    int res = fat16_release_cluster(disk, cluster);
    if (res < 0)
        return res;

    if (next_cluster < 0xFF8 && next_cluster != 0x00)
        return fat16_release_chain(disk, next_cluster);
    return ALL_OK;
}

static int fat16_set_end_of_chain(struct disk *disk, int cluster)
{
    struct fat_private *private = disk->fs_private;
    struct disk_stream *stream = private->fat_write_stream;
    if (!stream)
        return -EIO;

    u32 fat_table_pos = fat16_get_first_fat_sector(private) * disk->sector_size;
    if (disk_streamer_seek(stream, fat_table_pos * (cluster * FAT16_FAT_ENTRY_SIZE)) < 0)
        return -EIO;

    int res = fat16_release_chain(disk, cluster);
    if (res < 0)
        return res;

    u16 result = 0xFFF;
    if (disk_streamer_write(stream, &result, sizeof(result)) < 0)
        return -EIO;
    return ALL_OK;
}

static int fat16_get_free_cluster(struct disk* disk)
{
    struct fat_private* private = disk->fs_private;
    struct disk_stream* stream = private->fat_read_stream;
    if (!stream)
        return -EIO;

    u32 fat_table_pos = fat16_get_first_fat_sector(private) * disk->sector_size;
    int fat_table_size = private->header.primary_header.sectors_per_fat * disk->sector_size;
    int total_clusters = fat_table_size / FAT16_FAT_ENTRY_SIZE;
    u16* fat_table = kzalloc(fat_table_size);
    if (!fat_table)
        return -ENOMEM;

    if (disk_streamer_seek(stream, fat_table_pos) < 0)
        return -EIO;

    if (disk_streamer_read(stream, fat_table, fat_table_size) < 0)
        return -EIO;

    for (int i = 0; i < total_clusters; i++)
    {
        if (fat_table[i] == 0x00)
        {
            kfree(fat_table);
            return i;
        }
    }
    kfree(fat_table);
    return -ENOSPC;

}

static int fat16_update_fat_item_data(struct disk *disk, struct fat_dir_t *item, u32 position)
{
    struct fat_private *private = disk->fs_private;
    struct disk_stream *stream = private->directory_stream;
    if (!stream)
        return -EIO;

    int root_dir_size = private->header.primary_header.root_dir_entries * sizeof(struct fat_dir_t);
    if (disk_streamer_seek(stream, private->root_directory.sector_pos * disk->sector_size) < 0)
        return -EIO;

    if (disk_streamer_write(stream, private->root_directory.items, root_dir_size) < 0)
        return -EIO;
    return ALL_OK;
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

static int fat16_write_internal_to_stream(struct disk *disk, struct disk_stream *stream, int cluster, int offset, void *buffer, int size)
{
    struct fat_private *private = disk->fs_private;
    int cluster_size = private->header.primary_header.sectors_per_cluster * disk->sector_size;
    int cluster_use = fat16_get_cluster_for_offset(disk, cluster, offset);
    if (cluster_use < 0)
        return cluster_use;

    int cluster_offset = offset % cluster_size;

    int starting_sector = fat16_cluster_to_sector(private, cluster_use);
    int starting_pos = (starting_sector * disk->sector_size) + cluster_offset;
    int total_to_write = size > cluster_size ? cluster_size : size;
    int res = disk_streamer_seek(stream, starting_pos);
    if (res != ALL_OK)
        return res;

    res = disk_streamer_write(stream, buffer, total_to_write);
    if (res != ALL_OK)
        return res;

    size -= total_to_write;
    if (size > 0)
        return fat16_write_internal_to_stream(disk, stream, cluster, offset + total_to_write, buffer + total_to_write, size);
    return ALL_OK;
}

static int fat16_read_internal(struct disk *disk, int starting_cluster, int offset, void *buffer, int size)
{
    struct fat_private *private = disk->fs_private;
    struct disk_stream *stream = private->cluser_read_stream;
    return fat16_read_internal_from_stream(disk, stream, starting_cluster, offset, buffer, size);
}

static int fat16_write_internal(struct disk *disk, int starting_cluster, int offset, void *buffer, int size)
{
    struct fat_private *private = disk->fs_private;
    struct disk_stream *stream = private->fat_write_stream;
    return fat16_write_internal_to_stream(disk, stream, starting_cluster, offset, buffer, size);
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

static struct fat_directory *fat16_load_fat_directory(struct disk *disk, struct fat_dir_t *item)
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
    int directory_size = total_items * sizeof(struct fat_dir_t);
    directory->items = kzalloc(directory_size);
    directory->ending_sector_pos = 0;
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

static struct fat_item *fat16_new_fat_item_for_directory_item(struct disk *disk, struct fat_dir_t *item)
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
    f_item->item = fat16_clone_directory_item(item, sizeof(struct fat_dir_t));
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
    struct fat_file_descriptor *descriptor = kzalloc(sizeof(struct fat_file_descriptor));
    if (!descriptor)
        return ERROR(-ENOMEM);

    descriptor->item = fat16_get_directory_entry(disk, path);
    if (!descriptor->item && mode != FILE_MODE_WRITE)
    {
        kfree(descriptor);
        return ERROR(-EBADPATH);
    } else if (!descriptor->item)
    {
        descriptor->item = NULL;
    }

    if (mode == FILE_MODE_APPEND) {
        descriptor->pos = descriptor->item->item->filesize;
    } else {
        descriptor->pos = 0;
    }
    descriptor->mode = mode;
    return descriptor;
}

int fat16_read(struct disk *disk, void *descriptor, u32 size, void *out_ptr)
{
    struct fat_file_descriptor *desc = descriptor;
    struct fat_dir_t *item = desc->item->item;
    int offset = desc->pos;
    int res = fat16_read_internal(disk, fat32_get_first_cluster(item), offset, out_ptr, size);
    if (res < 0)
        return res;

    return size;
}

int fat16_seek(void *private, u32 offset, FILE_SEEK_MODE mode)
{
    struct fat_file_descriptor *desc = private;
    struct fat_item *item = desc->item;
    if (item->type != FAT_ITEM_TYPE_FILE)
        return -EINVARG;

    struct fat_dir_t *dir_item = item->item;

    switch (mode)
    {
    case FILE_SEEK_SET:
        desc->pos = offset;
        break;
    case FILE_SEEK_CUR:
        desc->pos += offset;
        break;
    case FILE_SEEK_END:
        desc->pos = dir_item->filesize;
        break;
    default:
        return -EINVARG;
        break;
    }
    if (desc->pos > dir_item->filesize)
        return -EIO;
    return ALL_OK;
}

int fat16_mkdir(struct disk *disk, struct path_part *path)
{
    return -ERDONLY;
}

int fat16_write(struct disk *disk, void *descriptor, u32 size, u32 nmemb, void *in_ptr)
{
    struct fat_file_descriptor *desc = descriptor;
    if (desc->mode == FILE_MODE_READ)
        return -ERDONLY;

    if (desc->item == NULL)
        return -EINVARG;

    struct fat_dir_t *item = desc->item->item;
    // int offset = desc->pos;
    int cluster = fat32_get_first_cluster(item);
    if (cluster == 0)
    {
        cluster = fat16_get_free_cluster(disk);
        if (cluster < 0)
            return cluster;

        item->first_cluster_high = (cluster >> 16) & 0xFFFF;
        item->first_cluster_low = cluster & 0xFFFF;
    }
    // int res
    return -EUNIMP;
}

int fat16_stat(struct disk *disk, void *private, struct file_stat *stat)
{
    struct fat_file_descriptor *desc = private;
    struct fat_item *item = desc->item;
    if (item->type != FAT_ITEM_TYPE_FILE)
        return -EINVARG;

    struct fat_dir_t *dir_item = item->item;
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

void print_fat16_tree(struct fat_directory *directory,struct disk *disk,int depth) {
    for (int i = 0; i < directory->total; i++) {
        struct fat_dir_t *item = &directory->items[i];
        if (item->attributes & FAT_FILE_SYSTEM) {
            continue;
        }
        if (strncmp((const char *)item->filename, ".", 1) == 0 || strncmp((const char *)item->filename, "..", 2) == 0) {
            continue;
        }
        for (int j = 0; j < depth; j++) {
            serial_printf("\t");
        }
        char filename[9];
        char ext[4];
        memcpy(filename, item->filename, 8);
        filename[8] = 0;
        memcpy(ext, item->ext, 3);
        ext[3] = 0;

        // lowercase
        for (int i = 0; i < 8; i++) {
            if (filename[i] >= 'A' && filename[i] <= 'Z') {
                filename[i] += 32;
            }else if (filename[i] == 0x20) {
                filename[i] = 0;
            }
        }

        for (int i = 0; i < 3; i++) {
            if (ext[i] >= 'A' && ext[i] <= 'Z') {
                ext[i] += 32;
            }else if (ext[i] == 0x20) {
                ext[i] = 0;
            }
        }

        serial_printf("%s", filename);
        if (ext[0] != 0x0) {
            serial_printf(".%s", ext);
        }

        if (item->attributes & FAT_FILE_SUBDIRECTORY) {
            serial_printf("/\n");
            struct fat_directory *subdirectory = fat16_load_fat_directory(disk, item);
            if (subdirectory) {
                print_fat16_tree(subdirectory, disk,depth + 1); 
                fat16_free_directory(subdirectory);
            }
        }
        serial_printf("\n");
    }
}

void tree_fat16(struct disk *disk)
{
    struct fat_private *private = disk->fs_private;
    struct fat_directory *root = &private->root_directory;
    serial_printf("Root Directory:\n");
    print_fat16_tree(root, disk,0);
    serial_printf("\n");
}