#include <os/streamer.h>
#include <os/types.h>
#include <os/kheap.h>
#include <os/memory.h>
#include <os/config.h>
#include <os/kernel.h>


static struct cache cache[MAX_DISKS];

void disk_streamer_init()
{
    for (int i = 0; i < MAX_DISKS; i++)
    {
        cache[i].sector = -1;
        cache[i].dirty = false;

    }
}

struct disk_stream *disk_streamer_new(int disk_id)
{
    struct disk *disk = disk_get(disk_id);
    if (!disk)
    {
        return NULL;
    }

    struct disk_stream *streamer = kzalloc(sizeof(struct disk_stream));
    streamer->disk = disk;
    streamer->pos = 0;
    streamer->cache = &cache[disk_id];
    return streamer;
}

int disk_streamer_flush(struct disk_stream *streamer)
{
    if (streamer->cache->dirty && streamer->cache->sector != -1)
    {
        streamer->cache->dirty = false;
        int timeout = 5;
        int res = -1;
        do {
            res = disk_write_block(streamer->disk, streamer->cache->sector, 1, streamer->cache->data);
        } while (res < 0 && timeout-- > 0);

        if (res < 0){
            return res;
        }
    }
    return 0;
}

int disk_streamer_seek(struct disk_stream *streamer, int pos)
{
    streamer->pos = pos;
    return 0;
}

static int disk_streamer_read_sector(struct disk_stream *streamer, int sector)
{
    if (streamer->cache->sector == sector)
        return 0;
    int res = disk_streamer_flush(streamer);
    if (res < 0)
    {
        kernel_panic("Failed to flush cache\n");
    }
    int timeout = 5;
    do {
        res = disk_read_block(streamer->disk, sector, 1, streamer->cache->data);
    } while (res < 0 && timeout-- > 0);
    if (res < 0)
    {
        streamer->cache->sector = -1;
        return res;
    }
    streamer->cache->sector = sector;
    return 0;
}

int disk_streamer_read(struct disk_stream *streamer, void *out, int total)
{
    int sector = streamer->pos / SECTOR_SIZE;
    int offset = streamer->pos % SECTOR_SIZE;
    int total_to_read = total;
    bool overflow = (offset + total_to_read) >= SECTOR_SIZE;

    if (overflow)
    {
        total_to_read -= (offset + total_to_read) - SECTOR_SIZE;
    }

    int res = disk_streamer_read_sector(streamer, sector);
    if (res < 0)
    {
        return res;
    }

    for (int i = 0; i < total_to_read; i++)
    {
        *(char *)out++ = streamer->cache->data[offset + i];
    }

    streamer->pos += total_to_read;
    if (overflow)
    {
        return disk_streamer_read(streamer, out, total - total_to_read);
    }
    return 0;
}

int disk_streamer_write(struct disk_stream *streamer, void *buf, int total)
{
    int sector = streamer->pos / SECTOR_SIZE;
    int offset = streamer->pos % SECTOR_SIZE;
    int total_to_write = total;
    bool overflow = (offset + total_to_write) >= SECTOR_SIZE;
    int remaining = total;

    if (overflow)
    {
        total_to_write -= (offset + total_to_write) - SECTOR_SIZE;
    }

    int res = disk_streamer_read_sector(streamer, sector);
    if (res < 0)
    {
        return res;
    }

    for (int i = 0; i < total_to_write; i++)
    {
        streamer->cache->data[offset + i] = *(char *)buf++;
        streamer->cache->dirty = true;
    }

    streamer->pos += total_to_write;
    if (overflow)
    {
        return disk_streamer_write(streamer, buf, remaining - total_to_write);
    }
    return 0;
}

void disk_streamer_close(struct disk_stream *streamer)
{
    kfree(streamer);
}