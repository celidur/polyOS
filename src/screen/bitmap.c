#include <os/bitmap.h>
#include <os/terminal.h>
#include <os/file.h>
#include <os/vga.h>
#include <os/kheap.h>

bitmap_t * bitmap_create(char * filename) {
    bitmap_t * ret = kmalloc(sizeof(bitmap_t));
    int fd = fopen(filename, "r");
    if(!fd) {
        serial_printf("Fail to open %s\n", filename);
        return NULL;
    }

    struct file_stat stat;
    if(fstat(fd, &stat) < 0) {
        serial_printf("Fail to stat %s\n", filename);
        fclose(fd);
        return NULL;
    }

    void * buf = kmalloc(stat.size);
    if(fread(buf, stat.size, fd) != stat.size) {
        serial_printf("Fail to read %s\n", filename);
        kfree(buf);
        fclose(fd);
        return NULL;
    }


    // Parse the bitmap
    bmp_fileheader_t * h = buf;
    unsigned int offset = h->bfOffBits;

    bmp_infoheader_t * info = buf + sizeof(bmp_fileheader_t);

    ret->width = info->biWidth;
    ret->height = info->biHeight;
    ret->image_bytes= (void*)((unsigned int)buf + offset);
    ret->buf = buf;
    ret->total_size= stat.size;
    ret->bpp = info->biBitCount;
    fclose(fd);
    return ret;
}


void display_monochrome_bitmap(bitmap_t *bitmap) {
    if (!bitmap || !bitmap->image_bytes || bitmap->bpp != 1) {
        // Invalid bitmap or not monochrome.
        return;
    }

    uint8_t *pixelData = (uint8_t *)bitmap->image_bytes;
    unsigned int width = bitmap->width;
    unsigned int height = bitmap->height;

    for (unsigned int y = 0; y < height; y++) {
        for (unsigned int x = 0; x < width; x++) {
            unsigned int byteIndex = (y * width + x) / 8;
            unsigned int bitIndex = x % 8;
            uint8_t pixel = (pixelData[byteIndex] >> (7 - bitIndex)) & 1;

            // Choose color: 0 for black, 0xFFFFFF for white.
            uint32_t color = pixel ? 0xFFFFFF : 0;

            set_pixel(x, height-y, color);
        }
    }
}

void free_bitmap(bitmap_t *bitmap) {
    if (bitmap) {
        kfree(bitmap->buf);
        kfree(bitmap);
    }
}