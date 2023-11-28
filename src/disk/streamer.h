#ifndef DISKSTREAMER_H
#define DISKSTREAMER_H

#include "disk.h"
#include <stddef.h>

struct disk_stream
{
    int pos;
    struct disk *disk;
};

struct disk_stream *diskstreamer_new(int disk_id);
int diskstreamer_seek(struct disk_stream *streamer, int pos);
int diskstreamer_read(struct disk_stream *streamer, void *buf, int total);
void diskstreamer_close(struct disk_stream *streamer);

#endif