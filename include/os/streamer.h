#ifndef DISK_STREAMER_H
#define DISK_STREAMER_H

#include <os/disk.h>
#include <os/config.h>

struct cache
{
    int sector;
    char data[SECTOR_SIZE];
    bool dirty;
};

struct disk_stream
{
    int pos;
    struct disk *disk;
    struct cache *cache;
};

struct disk_stream *disk_streamer_new(int disk_id);
int disk_streamer_seek(struct disk_stream *streamer, int pos);
int disk_streamer_read(struct disk_stream *streamer, void *buf, int total);
int disk_streamer_write(struct disk_stream *streamer, void *buf, int total);
void disk_streamer_close(struct disk_stream *streamer);
int disk_streamer_flush(struct disk_stream *streamer);
void disk_streamer_init();

#endif