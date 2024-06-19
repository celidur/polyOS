#ifndef FAT16_H
#define FAT16_H

#include <os/file.h>
#include <os/config.h>

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
#define FAT_ITEM_TYPE_FILE 0x03

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


struct fat_dir_t;
struct fat_root_dir_t;
struct fat_entry_t;
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

struct fat_dir_item_t
{
    struct fat_item item;
    struct fat_dir_item_t *next;
    struct fat_dir_item_t *prev;
};

struct fat_entry_t
{
    struct fat_entry_alias alias;
    char filename[MAX_FILENAME];
    u32 entry_offset;
    u32 nb_entries;
    struct fat_item* parent;
};

struct fat_dir_root_item_t
{
    struct fat_dir_item_t *items;
    int nb_used; // to free only if not used
};

struct fat_dir_t
{
    struct fat_entry_t self;
    struct fat_dir_root_item_t *items;
};

struct fat_root_dir_t
{
    struct fat_dir_root_item_t *items;
    int sector_pos;
    int ending_sector_pos;
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
    int sector_size;

    struct disk_stream *cluser_read_stream;
    struct disk_stream *fat_read_stream;
    struct disk_stream *fat_write_stream;
    struct disk_stream *directory_stream;
};

static int fat16_resolve(struct disk *disk);
static void *fat16_open(void *fs_private, struct path_part *path, FILE_MODE mode);
static int fat16_read(void *fs_private, void *descriptor, u32 size, void *out_ptr);
static int fat16_seek(void *fd_private, u32 offset, FILE_SEEK_MODE mode);
static int fat16_stat(void *fd_private, struct file_stat *stat);
static int fat16_close(void *private);
static void fat16_tree(void *fs_private);
static int fat16_write(void *fd_private, void *descriptor, u32 size, void *in_ptr);

#endif