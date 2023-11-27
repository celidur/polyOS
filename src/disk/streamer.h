#ifndef DISKSTREAMER_H
#define DISKSTREAMER_H

#include "disk.h"
#include <stddef.h>

struct disk_streamer
{
    int pos;
    struct disk *disk;
};

struct disk_streamer *diskstreamer_new(int disk_id);
int diskstreamer_seek(struct disk_streamer *streamer, int pos);
int diskstreamer_read(struct disk_streamer *streamer, void *buf, int total);
void diskstreamer_close(struct disk_streamer *streamer);

#endif