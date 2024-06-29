#include "stdio.h"
#include "string.h"

int main (int argc, char** argv) {
    if (argc < 2) {
        printf("Usage: %s <filename>\n", argv[0]);
        return 1;
    }

    char filename[1024];
    strcpy(filename, "0:");
    strncpy(filename + 2, argv[1], 1022);

    int fd = fopen(filename, "r");
    if (fd < 0) {
        printf("Failed to open file: %d\n", fd);
        return 1;
    }
    char* buff[1024];
    int res = fread(fd, buff, 1024);
    if (res < 0) {
        printf("Failed to read file\n");
        return 1;
    }
    printf("File content: %s\n", buff);
    fclose(fd);
    
    return 0;
}