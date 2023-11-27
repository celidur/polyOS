#include "streamer.h"
#include "memory/heap/kheap.h"
#include "memory/memory.h"
#include "config.h"

struct disk_streamer *diskstreamer_new(int disk_id)
{
    struct disk *disk = disk_get(disk_id);
    if (!disk)
    {
        return NULL;
    }

    struct disk_streamer *streamer = kzalloc(sizeof(struct disk_streamer));
    streamer->disk = disk;
    streamer->pos = 0;
    return streamer;
}

int diskstreamer_seek(struct disk_streamer *streamer, int pos)
{
    streamer->pos = pos;
    return 0;
}

int diskstreamer_read(struct disk_streamer *streamer, void *out, int total)
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
    memcpy(out, buf + offset, total_to_read);

    streamer->pos += total_to_read;
    if (total > SECTOR_SIZE)
    {
        return diskstreamer_read(streamer, out, total - SECTOR_SIZE);
    }
    return 0;
}

void diskstreamer_close(struct disk_streamer *streamer)
{
    kfree(streamer);
}