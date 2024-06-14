#include "fat16.h"
#include <os/disk.h>
#include <os/string.h>
#include <os/status.h>
#include <os/streamer.h>
#include <os/kheap.h>
#include <os/types.h>
#include <os/memory.h>

// check https://github.com/eerimoq/simba/blob/master/src/filesystems/fat16.c

#include <os/terminal.h>
#include <os/string.h>


struct filesystem fat16_fs = {
    .resolve = fat16_resolve,
    .open = fat16_open,
    .read = fat16_read,
    .seek = fat16_seek,
    .stat = fat16_stat,
    .close = fat16_close,
    .tree = fat16_tree,
    .name = "FAT16"
};

struct filesystem *fat16_init()
{
    return &fat16_fs;
}

static void fat16_init_private(struct disk *disk, struct fat_private *private)
{
    memset(private, 0, sizeof(struct fat_private));
    private->cluser_read_stream = disk_streamer_new(disk->id);
    private->fat_read_stream = disk_streamer_new(disk->id);
    private->directory_stream = disk_streamer_new(disk->id);
    private->fat_write_stream = disk_streamer_new(disk->id);
    private->sector_size = disk->sector_size;
}

static inline bool fat16_entry_is_long_name(const struct fat_entry_alias *alias)
{
    return (alias->attributes & FAT_DIR_ATTR_LONG_NAME_MASK) == FAT_DIR_ATTR_LONG_NAME;
}

static inline bool fat16_is_directory(const struct fat_item *item)
{
    return (item->type == FAT_ITEM_TYPE_SUBDIRECTORY || item->type == FAT_ITEM_TYPE_ROOT_DIRECTORY);
}

static inline int fat16_sector_to_absolute(struct fat_private *private, int sector)
{
    return sector * private->sector_size;
}

static inline u32 fat32_get_first_cluster(struct fat_entry_t *item)
{
    return (item->alias.first_cluster_high << 16) | item->alias.first_cluster_low;
}

static inline int fat16_cluster_to_sector(struct fat_private *private, int cluser)
{
    return private->root_directory.root_dir->ending_sector_pos + ((cluser - 2) * private->header.primary_header.sectors_per_cluster);
}

static inline int fat16_get_cluster_size(struct fat_private *private)
{
    return private->header.primary_header.sectors_per_cluster * private->sector_size;
}

static inline u32 fat16_get_first_fat_sector(struct fat_private *private)
{
    return private->header.primary_header.reserved_sectors;
}

static int fat16_get_next_cluster(struct fat_private *private, int cluster)
{
    struct disk_stream *stream = private->fat_read_stream;
    if (!stream)
        return -EIO;

    u32 fat_table_pos = fat16_get_first_fat_sector(private) * private->sector_size;
    if (disk_streamer_seek(stream, fat_table_pos * (cluster * FAT16_FAT_ENTRY_SIZE)) < 0)
        return -EIO;

    u16 result = 0;
    if (disk_streamer_read(stream, &result, sizeof(result)) < 0)
        return -EIO;
    return result;
}

static int fat16_get_cluster_for_offset(struct fat_private *private, int starting_cluster, int offset)
{
    int cluster = starting_cluster;
    int cluster_ahead = offset / fat16_get_cluster_size(private);
    for (int i = 0; i < cluster_ahead; i++)
    {
        int entry = fat16_get_next_cluster(private, cluster);
        if (entry >= FAT16_BAD_SECTOR || entry == FAT16_UNUSED)
            return -EIO;
        cluster = entry;
    }
    return cluster;
}

static int fat16_read_internal_from_stream(struct fat_private *private, struct disk_stream *stream, int cluster, int offset, void *buffer, int size)
{
    int cluster_size = fat16_get_cluster_size(private);
    int cluster_use = fat16_get_cluster_for_offset(private, cluster, offset);
    if (cluster_use < 0)
        return cluster_use;

    int cluster_offset = offset % cluster_size;

    int starting_sector = fat16_cluster_to_sector(private, cluster_use);
    int starting_pos = (starting_sector * private->sector_size) + cluster_offset;
    int total_to_read = size > cluster_size ? cluster_size : size;
    int res = disk_streamer_seek(stream, starting_pos);
    if (res != ALL_OK)
        return res;

    res = disk_streamer_read(stream, buffer, total_to_read);
    if (res != ALL_OK)
        return res;

    size -= total_to_read;
    if (size > 0)
        return fat16_read_internal_from_stream(private, stream, cluster, offset + total_to_read, buffer + total_to_read, size);
    return ALL_OK;
}

static int fat16_read_internal(struct fat_private *private, int starting_cluster, int offset, void *buffer, int size)
{
    struct disk_stream *stream = private->cluser_read_stream;
    return fat16_read_internal_from_stream(private, stream, starting_cluster, offset, buffer, size);
}

static int fat16_make_name_from_83(struct fat_entry_alias* alias, char* buff) {
    if (!alias || !buff)
        return -EINVARG;
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
    if (fat16_entry_is_long_name(&data->alias)){
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
    memcpy(&entry->alias, &data->alias, sizeof(struct fat_entry_alias));
    return 2;
}

static int fat16_get_entry(struct fat_private *fat_private, struct fat_item *current_directory,int offset ,struct raw_data *entry) {
    if (!current_directory || !entry || !fat16_is_directory(current_directory))
        return -EINVARG;

    if (current_directory->type == FAT_ITEM_TYPE_ROOT_DIRECTORY) {
        struct fat_header *header = &fat_private->header.primary_header;
        struct disk_stream *stream = fat_private->directory_stream;
        int root_directory_sector_pos = (header->fat_copies * header->sectors_per_fat) + header->reserved_sectors;
        if (disk_streamer_seek(stream, fat16_sector_to_absolute(fat_private, root_directory_sector_pos) + offset) != ALL_OK)
            return -EIO;

        if (disk_streamer_read(stream, entry, sizeof(struct raw_data)) != ALL_OK)
            return -EIO;
    } else {
        int cluster = fat32_get_first_cluster(&current_directory->directory->self);
        if (fat16_read_internal(fat_private, cluster, offset, entry, sizeof(struct raw_data)) != ALL_OK)
            return -EIO;
    }

    return 0;
}

static int fat16_load_directory_entries(struct fat_private *fs_private, struct fat_item *parent, struct fat_item *current_directory)
{
    if (!parent || !current_directory || !fat16_is_directory(current_directory) || !fat16_is_directory(parent))
        return -EINVARG;
    
    int res = 0;
    struct fat_dir_item_t *start = NULL;
    struct fat_dir_item_t *current = NULL;
    struct fat_entry_t *entry = NULL;
    struct raw_data data;
    int offset = 0;
    while (true)
    {
        res = fat16_get_entry(fs_private, current_directory, offset, &data);
        if (res != 0)
            break;
        offset += sizeof(struct raw_data);
        struct fat_entry_alias *dir = &data.alias;
        if (dir->filename[0] == 0x00)
            break;
        if (dir->filename[0] == 0xE5)
            continue;
        if (!entry){
            entry = kzalloc(sizeof(struct fat_entry_t));
            if (!entry)
                return -ENOMEM;
            entry->entry_offset = offset;
            entry->nb_entries = 0;
        }
        entry->nb_entries++;
        res = fat16_read_entry(&data, entry);
        if (res == 2){
            if (!start){
                start = kzalloc(sizeof(struct fat_dir_item_t));
                if (!start)
                    return -ENOMEM;
                current = start;
            } else {
                current->next = kzalloc(sizeof(struct fat_dir_item_t));
                if (!current->next)
                    return -ENOMEM;
                current->next->prev = current;
                current = current->next;
            }
            struct fat_item item;
            item.type = entry->alias.attributes & FAT_FILE_SUBDIRECTORY ? FAT_ITEM_TYPE_SUBDIRECTORY : FAT_ITEM_TYPE_FILE;
            if (item.type == FAT_ITEM_TYPE_SUBDIRECTORY){
                item.directory = kzalloc(sizeof(struct fat_dir_t));
                if (!item.directory)
                    return -ENOMEM;
                memcpy(&item.directory->self, entry, sizeof(struct fat_entry_t));
                // only free for directories
                kfree(entry);
                item.directory->items = NULL;
                // verify if is . or ..
                if (item.directory->self.filename[0] == 0x2E){
                    if (item.directory->self.filename[1] == 0x00){
                        item.directory->items = current_directory->directory->items;
                        current_directory->directory->items->nb_used++;
                        goto next;
                    } else if (item.directory->self.filename[1] == 0x2E){
                        item.directory->items = parent->directory->items;
                        parent->directory->items->nb_used++;
                        goto next;
                    }
                }

            
                if (fat16_load_directory_entries(fs_private, current_directory, &item) < 0) {
                    return -EIO;
                }

            } else {
                item.type = FAT_ITEM_TYPE_FILE;
                item.item = entry;
            }
next:
            memcpy(&current->item, &item, sizeof(struct fat_item));
            entry = NULL;
        }
    }
    if (entry)
        kfree(entry);

    if (start){
        struct fat_dir_root_item_t *root = kzalloc(sizeof(struct fat_dir_root_item_t));
        if (!root)
            return -ENOMEM;
        root->items = start;
        root->nb_used = 1;
        if (current_directory->type == FAT_ITEM_TYPE_ROOT_DIRECTORY)
            current_directory->root_dir->items = root;
        else
            current_directory->directory->items = root;
    }

    return ALL_OK;
}

// TODO: Clean memory
static int fat16_get_root_directory(struct fat_private *fs_private)
{
    struct fat_item *root = &fs_private->root_directory;
    struct fat_root_dir_t *directory = kzalloc(sizeof(struct fat_root_dir_t));
    if (!directory)
        return -ENOMEM;

    root->type = FAT_ITEM_TYPE_ROOT_DIRECTORY;
    root->root_dir = directory;
    struct fat_header *header = &fs_private->header.primary_header;
    int root_directory_sector_pos = (header->fat_copies * header->sectors_per_fat) + header->reserved_sectors;
    int root_dir_entries = fs_private->header.primary_header.root_dir_entries;
    int root_dir_size = root_dir_entries * sizeof(struct raw_data);
    directory->sector_pos = root_directory_sector_pos;
    directory->ending_sector_pos = root_directory_sector_pos + (root_dir_size / fs_private->sector_size);

    int res = fat16_load_directory_entries(fs_private, root, root);
    if (res < 0)
        return res;
    
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

    if (fat16_get_root_directory(private) != ALL_OK)
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

static int fat16_release_cluster(struct fat_private *private, int cluster)
{
    struct disk_stream *stream = private->fat_write_stream;
    if (!stream)
        return -EIO;

    u32 fat_table_pos = fat16_get_first_fat_sector(private) * private->sector_size;
    if (disk_streamer_seek(stream, fat_table_pos * (cluster * FAT16_FAT_ENTRY_SIZE)) < 0)
        return -EIO;

    u16 result = FAT16_UNUSED;
    if (disk_streamer_write(stream, &result, sizeof(result)) < 0)
        return -EIO;
    return ALL_OK;
}

static int fat16_release_chain(struct fat_private *private, int cluster)
{
    int next_cluster = fat16_get_next_cluster(private, cluster);
    if (next_cluster < 0)
        return next_cluster;

    int res = fat16_release_cluster(private, cluster);
    if (res < 0)
        return res;

    if (next_cluster < FAT16_BAD_SECTOR || next_cluster != FAT16_UNUSED)
        return fat16_release_chain(private, next_cluster);
    return ALL_OK;
}

static int fat16_set_end_of_chain(struct fat_private *private, int cluster)
{
    struct disk_stream *stream = private->fat_write_stream;
    if (!stream)
        return -EIO;

    u32 fat_table_pos = fat16_get_first_fat_sector(private) * private->sector_size;
    if (disk_streamer_seek(stream, fat_table_pos * (cluster * FAT16_FAT_ENTRY_SIZE)) < 0)
        return -EIO;

    int res = fat16_release_chain(private, cluster);
    if (res < 0)
        return res;

    u16 result = FAT16_EOC;
    if (disk_streamer_write(stream, &result, sizeof(result)) < 0)
        return -EIO;
    return ALL_OK;
}

static int fat16_get_free_cluster(struct fat_private *private)
{
    struct disk_stream* stream = private->fat_read_stream;
    if (!stream)
        return -EIO;

    u32 fat_table_pos = fat16_get_first_fat_sector(private) * private->sector_size;
    int fat_table_size = private->header.primary_header.sectors_per_fat * private->sector_size;
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
        if (fat_table[i] == FAT16_UNUSED)
        {
            kfree(fat_table);
            return i;
        }
    }
    kfree(fat_table);
    return -ENOSPC;
}

// TODO: Implement
static int fat16_update_fat_item_data(struct fat_private *private, struct fat_entry_t *item)
{
    struct disk_stream *stream = private->directory_stream;
    if (!stream)
        return -EIO;

    int filename_size = strnlen(item->filename, MAX_FILENAME);
    int nb_entries = filename_size / 13 + (filename_size % 13 == 0 ? 0 : 1) + 1;
    int total_size = nb_entries * sizeof(struct raw_data);
    struct raw_data *data = kzalloc(total_size);
    for (int i = 0; i < nb_entries - 1; i++)
    {
        u8 *entry = data[i].data;
        entry[1]  = item->filename[i + 0];  
        entry[3]  = item->filename[i + 1];  
        entry[5]  = item->filename[i + 2];  
        entry[7]  = item->filename[i + 3];  
        entry[9]  = item->filename[i + 4];  
        entry[14] = item->filename[i + 5];
        entry[16] = item->filename[i + 6];
        entry[18] = item->filename[i + 7];
        entry[20] = item->filename[i + 8];
        entry[22] = item->filename[i + 9];
        entry[24] = item->filename[i + 10];
        entry[28] = item->filename[i + 11];
        entry[30] = item->filename[i + 12];
    }
    memcpy(&data[nb_entries - 1].alias, &item->alias, sizeof(struct fat_entry_alias));
    // u32 offset = item->entry_offset;
    if (item->nb_entries > nb_entries)
    {
        serial_printf("Need to realease some entries\n");
        // u32 nb_entries_to_release = item->nb_entries - nb_entries;

    } else if (item->nb_entries < nb_entries)
    {
        serial_printf("Need to add some entries\n");
    }

    serial_printf("Need to update the fat table\n");

    kfree(data);

    return 0;
}


static int fat16_write_internal_to_stream(struct fat_private *private, struct disk_stream *stream, int cluster, int offset, void *buffer, int size)
{
    // struct fat_private *private = disk->fs_private;
    // int cluster_size = private->header.primary_header.sectors_per_cluster * disk->sector_size;
    // int cluster_use = fat16_get_cluster_for_offset(disk, cluster, offset);
    // if (cluster_use < 0)
    //     return cluster_use;

    // int cluster_offset = offset % cluster_size;

    // int starting_sector = fat16_cluster_to_sector(private, cluster_use);
    // int starting_pos = (starting_sector * disk->sector_size) + cluster_offset;
    // int total_to_write = size > cluster_size ? cluster_size : size;
    // int res = disk_streamer_seek(stream, starting_pos);
    // if (res != ALL_OK)
    //     return res;

    // res = disk_streamer_write(stream, buffer, total_to_write);
    // if (res != ALL_OK)
    //     return res;

    // size -= total_to_write;
    // if (size > 0)
    //     return fat16_write_internal_to_stream(disk, stream, cluster, offset + total_to_write, buffer + total_to_write, size);
    return ALL_OK;
}

static int fat16_write_internal(struct fat_private *private, int starting_cluster, int offset, void *buffer, int size)
{
    struct disk_stream *stream = private->fat_write_stream;
    return fat16_write_internal_to_stream(private, stream, starting_cluster, offset, buffer, size);
}

static void fat16_free_item(struct fat_item *item);

static void fat16_free_directory_item(struct fat_dir_root_item_t *directory){
    if (!directory)
        return;
    
    struct fat_dir_item_t *current = directory->items;
    while (current)
    {
        struct fat_dir_item_t *tmp = current;
        current = current->next;
        fat16_free_item(&tmp->item);
        kfree(tmp);
    }
    directory->nb_used--;
    if (directory->nb_used == 0)
        kfree(directory);
}

static void fat16_free_item(struct fat_item *item)
{
    if (!item || item->type == FAT_ITEM_UNKNOWN)
        return;

    if (item->type == FAT_ITEM_TYPE_FILE && item->item)
    {
        kfree(item->item);
    }
    else if (item->type == FAT_ITEM_TYPE_SUBDIRECTORY && item->directory)
    {
        fat16_free_directory_item(item->directory->items);
        kfree(item->directory);
    }
    else if (item->type == FAT_ITEM_TYPE_ROOT_DIRECTORY && item->root_dir)
    {
        fat16_free_directory_item(item->root_dir->items);
        kfree(item->root_dir);
    }
}

static struct fat_item *fat16_find_item_in_directory(struct fat_dir_root_item_t *directory, const char *name)
{
    if (!directory || !name)
        return NULL;

    struct fat_dir_item_t *current = directory->items;
    while (current)
    {
        struct fat_item *item = &current->item;
        if (strncmp((const char *)item->item->filename, name, MAX_FILENAME) == 0)
            return item;
        current = current->next;
    }
    
    return NULL;
}

static struct fat_item *fat16_get_directory_entry(struct fat_private *private, struct path_part *path)
{
    struct fat_item *root = fat16_find_item_in_directory(private->root_directory.root_dir->items, path->part);
    if (!root)
        return NULL;

    struct fat_item *current_item = root;

    struct path_part *next_part = path->next;
    while (next_part != NULL)
    {
        if (current_item->type != FAT_FILE_SUBDIRECTORY && next_part->next != NULL)
            return NULL;

        current_item = fat16_find_item_in_directory(current_item->directory->items, next_part->part);
        if (!current_item)
            return NULL;

        next_part = next_part->next;
    }
    return current_item;
}

static void *fat16_open(void *fs_private, struct path_part *path, FILE_MODE mode)
{
    struct fat_file_descriptor *descriptor = kzalloc(sizeof(struct fat_file_descriptor));
    if (!descriptor)
        return ERROR(-ENOMEM);

    descriptor->item = fat16_get_directory_entry(fs_private, path);
    if (!descriptor->item && mode != FILE_MODE_WRITE)
    {
        kfree(descriptor);
        return ERROR(-EBADPATH);
    } else if (!descriptor->item)
    {
        descriptor->item = NULL;
        // TODO: Create file
    }

    if (descriptor->item->type != FAT_ITEM_TYPE_FILE)
    {
        // only files are supported
        kfree(descriptor);
        return ERROR(-EINVARG);
    }

    descriptor->pos = 0;
    descriptor->mode = mode;
    return descriptor;
}

static int fat16_read(void *fs_private, void *descriptor, u32 size, void *out_ptr)
{
    struct fat_file_descriptor *desc = descriptor;
    if (desc->item->type != FAT_ITEM_TYPE_FILE)
        return -EINVARG;
    struct fat_entry_t *item = desc->item->item;
    int offset = desc->pos;
    // TODO: verify the size 
    if (offset + size > item->alias.filesize)
        size = item->alias.filesize - offset;
    int res = fat16_read_internal(fs_private, fat32_get_first_cluster(item), offset, out_ptr, size);
    if (res < 0)
        return res;

    return size;
}

static int fat16_seek(void *private, u32 offset, FILE_SEEK_MODE mode)
{
    struct fat_file_descriptor *desc = private;
    struct fat_item *item = desc->item;
    if (item->type != FAT_ITEM_TYPE_FILE)
        return -EINVARG;

    struct fat_entry_alias *dir_item = &item->item->alias;

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

// static int fat16_mkdir(struct disk *disk, struct path_part *path)
// {
//     return -ERDONLY;
// }

static int fat16_write(void *descriptor, u32 size, u32 nmemb, void *in_ptr)
{
    // struct fat_file_descriptor *desc = descriptor;
    // if (desc->mode == FILE_MODE_READ)
    //     return -ERDONLY;

    // if (desc->item == NULL)
    //     return -EINVARG;

    // struct fat_entry_t *item = desc->item->item;
    // // int offset = desc->pos;
    // int cluster = fat32_get_first_cluster(item);
    // if (cluster == 0)
    // {
    //     cluster = fat16_get_free_cluster(disk);
    //     if (cluster < 0)
    //         return cluster;

    //     item->first_cluster_high = (cluster >> 16) & 0xFFFF;
    //     item->first_cluster_low = cluster & 0xFFFF;
    // }
    // int res
    return -EUNIMP;
}

static int fat16_stat(void *fd_private, struct file_stat *stat)
{
    struct fat_file_descriptor *desc = fd_private;
    struct fat_item *item = desc->item;
    if (item->type != FAT_ITEM_TYPE_FILE)
        return -EINVARG;

    struct fat_entry_t *dir_item = item->item;
    struct fat_entry_alias *alias = &dir_item->alias;
    stat->size = alias->filesize;
    stat->flags = 0;

    if (alias->attributes & FAT_FILE_READ_ONLY)
        stat->flags |= FILE_STAT_READ_ONLY;
    return ALL_OK;
}

static int fat16_close(void *private)
{
    kfree(private);
    return ALL_OK;
}

static void print_fat16_tree(struct fat_dir_root_item_t *directory,int depth) {
    if (!directory)
        return;
    struct fat_dir_item_t *current = directory->items;
    while (current)
    {
        struct fat_item *item = &current->item;
        current = current->next;
        if (item->type == FAT_ITEM_TYPE_FILE)
        {
            for (int i = 0; i < depth; i++)
                serial_printf("\t");
            serial_printf("%s\n", item->item->filename);
        }
        else if (item->type == FAT_ITEM_TYPE_SUBDIRECTORY)
        {
            for (int i = 0; i < depth; i++)
                serial_printf("\t");
            char* filename = item->directory->self.filename;
            serial_printf("%s\n", filename);
            if (filename[0] == 0x2E && filename[1] == 0x00) {
                continue;
            }
            if (filename[0] == 0x2E && filename[1] == 0x2E && filename[2] == 0x00)
                continue;
            print_fat16_tree(item->directory->items, depth + 1);
        }
    }
}

static void fat16_tree(void *private)
{
    struct fat_private *fs_private = private;
    struct fat_dir_root_item_t *root = fs_private->root_directory.root_dir->items;
    serial_printf("Root Directory:\n");
    print_fat16_tree(root,0);
    serial_printf("\n");
}