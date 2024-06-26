#include <os/disk.h>
#include <os/config.h>
#include <os/memory.h>
#include <os/status.h>
#include <os/io.h>
#include <os/types.h>
#include <os/streamer.h>

static struct disk disks[MAX_DISKS];

static int disk_read_sector(int lba, int total, void *buf)
{
    outb(0x1F6, (lba >> 24) | 0xE0);
    outb(0x1F2, total);
    outb(0x1F3, (unsigned char)(lba & 0xFF));
    outb(0x1F4, (unsigned char)((lba >> 8)));
    outb(0x1F5, (unsigned char)((lba >> 16)));
    outb(0x1F7, 0x20);

    unsigned short *ptr = (unsigned short *)buf;
    for (int b = 0; b < total; b++)
    {
        int timeout = 100000;
        do {
            if (timeout-- == 0) {
                return -EIO;
            }
        } while ((inb(0x1F7) & 0x08) == 0);

        for (int i = 0; i < 256; i++)
        {
            *ptr = inw(0x1F0);
            ptr++;
        }
    }
    return 0;
}

static int disk_write_sector(int lba, int total, void *buf)
{
    outb(0x1F6, (lba >> 24) | 0xE0);
    outb(0x1F2, total);
    outb(0x1F3, (unsigned char)(lba & 0xFF));
    outb(0x1F4, (unsigned char)((lba >> 8)));
    outb(0x1F5, (unsigned char)((lba >> 16)));
    outb(0x1F7, 0x30);

    unsigned short *ptr = (unsigned short *)buf;
    for (int b = 0; b < total; b++)
    {
        int timeout = 100000;
        do {
            if (timeout-- == 0) {
                return -EIO;
            }
        } while ((inb(0x1F7) & 0x08) == 0);

        for (int i = 0; i < 256; i++)
        {
            outw(0x1F0, *ptr);
            ptr++;
        }
    }
    return 0;
}

void disk_search_and_init()
{
    disk_streamer_init();
    memset(&disks[0], 0, sizeof(struct disk));
    disks[0].type = DISK_TYPE_REAL;
    disks[0].sector_size = SECTOR_SIZE;
    disks[0].fs = fs_resolve(&disks[0]);
    disks[0].id = 0;
}

struct disk *disk_get(int index)
{
    if (index >= 0 && index < MAX_DISKS)
    {
        return &disks[index];
    }
    return NULL;
}

static bool disk_validate(struct disk *disk)
{
    for (int i = 0; i < MAX_DISKS; i++)
    {
        if (disk == &disks[i])
        {
            return true;
        }
    }
    return false;
}

int disk_read_block(struct disk *idisk, unsigned int lba, int total, void *buf)
{
    if (!disk_validate(idisk))
    {
        return -EIO;
    }

    return disk_read_sector(lba, total, buf);
}

int disk_write_block(struct disk *idisk, unsigned int lba, int total, void *buf)
{
    if (!disk_validate(idisk))
    {
        return -EIO;
    }

    return disk_write_sector(lba, total, buf);
}