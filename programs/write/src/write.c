#include "stdio.h"
#include "string.h"

int main (int argc, char** argv) {
    if (argc < 3) {
        printf("Usage: %s <filename> <content>\n", argv[0]);
        return 1;
    }
    // add 0: before the file name
    char filename[1024];
    strncpy(filename, argv[1], 1023);

    int fd = fopen(filename, "w");
    if (fd < 0) {
        printf("Failed to open file: %d\n", fd);
        return 1;
    }
    
    int res = fwrite(fd, argv[2], strlen(argv[2]));
    if (res < 0) {
        printf("Failed to write file\n");
        return 1;
    }
    fclose(fd);
    
    return 0;
}