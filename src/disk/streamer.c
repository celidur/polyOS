#include <os/streamer.h>
#include <os/types.h>
#include <os/kheap.h>
#include <os/memory.h>
#include <os/config.h>

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
    return streamer;
}

int disk_streamer_seek(struct disk_stream *streamer, int pos)
{
    streamer->pos = pos;
    return 0;
}

int disk_streamer_read(struct disk_stream *streamer, void *out, int total)
{
    int sector = streamer->pos / SECTOR_SIZE;
    int offset = streamer->pos % SECTOR_SIZE;
    int total_to_read = total;
    bool overflow = (offset + total_to_read) >= SECTOR_SIZE;
    char buf[SECTOR_SIZE];

    if (overflow)
    {
        total_to_read -= (offset + total_to_read) - SECTOR_SIZE;
    }

    int res = disk_read_block(streamer->disk, sector, 1, buf);
    if (res < 0)
    {
        return res;
    }

    for (int i = 0; i < total_to_read; i++)
    {
        *(char *)out++ = buf[offset + i];
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
    char sector_buf[SECTOR_SIZE];
    int remaining = total;

    if (overflow)
    {
        total_to_write -= (offset + total_to_write) - SECTOR_SIZE;
    }

    // read the first sector to keep the data before the write
    int res = disk_read_block(streamer->disk, sector, 1, sector_buf);
    if (res < 0)
    {
        return res;
    }

    for (int i = 0; i < total_to_write; i++)
    {
        sector_buf[offset + i] = *(char *)buf++;
    }

    res = disk_write_block(streamer->disk, sector, 1, sector_buf);
    if (res < 0)
    {
        return res;
    }

    streamer->pos += total_to_write;
    remaining -= total_to_write;
    if (remaining >= SECTOR_SIZE)
    {
        int nb_sectors = remaining / SECTOR_SIZE;
        res = disk_write_block(streamer->disk, sector + 1, nb_sectors, buf);
        if (res < 0)
        {
            return res;
        }
        streamer->pos += nb_sectors * SECTOR_SIZE;
        remaining -= nb_sectors * SECTOR_SIZE;
    }

    // read the last sector to keep the data after the write
    if (remaining > 0)
    {
        sector = streamer->pos / SECTOR_SIZE;
        res = disk_read_block(streamer->disk, sector, 1, sector_buf);
        if (res < 0)
        {
            return res;
        }

        for (int i = 0; i < remaining; i++)
        {
            sector_buf[i] = *(char *)buf++;
        }

        res = disk_write_block(streamer->disk, sector, 1, sector_buf);
        if (res < 0)
        {
            return res;
        }
    }
    return 0;
}

void disk_streamer_close(struct disk_stream *streamer)
{
    kfree(streamer);
}