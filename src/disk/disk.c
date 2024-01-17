#include "disk.h"
#include "config.h"
#include "memory/memory.h"
#include "status.h"
#include "io/io.h"
#include <stdbool.h>

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
        char c = insb(0x1F7);
        while (!(c & 0x08))
        {
            c = insb(0x1F7);
        }

        for (int i = 0; i < 256; i++)
        {
            *ptr = insw(0x1F0);
            ptr++;
        }
    }
    return 0;
}

void disk_search_and_init()
{
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