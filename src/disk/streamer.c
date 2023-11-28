#include "streamer.h"
#include "memory/heap/kheap.h"
#include "memory/memory.h"
#include "config.h"

struct disk_stream *diskstreamer_new(int disk_id)
{
    struct disk *disk = disk_get(disk_id);
    if (!disk)
    {
        return NULL;
    }

    struct disk_stream *streamer = kzalloc(sizeof(struct disk_stream));
    streamer->disk = disk;
    streamer->pos = 0;
    return streamer;
}

int diskstreamer_seek(struct disk_stream *streamer, int pos)
{
    streamer->pos = pos;
    return 0;
}

int diskstreamer_read(struct disk_stream *streamer, void *out, int total)
{
    int sector = streamer->pos / SECTOR_SIZE;
    int offset = streamer->pos % SECTOR_SIZE;
    char buf[SECTOR_SIZE];

    int res = disk_read_block(streamer->disk, sector, 1, buf);
    if (res < 0)
    {
        return res;
    }

    int total_to_read = total > SECTOR_SIZE ? SECTOR_SIZE : total;

    for (int i = 0; i < total_to_read; i++)
    {
        *(char *)out++ = buf[offset + i];
    }

    streamer->pos += total_to_read;
    if (total > SECTOR_SIZE)
    {
        return diskstreamer_read(streamer, out, total - SECTOR_SIZE);
    }
    return 0;
}

void diskstreamer_close(struct disk_stream *streamer)
{
    kfree(streamer);
}