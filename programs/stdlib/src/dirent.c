#include "errno.h"
#include "memory.h"
#include "stdio.h"
#include "stdlib.h"

struct DIR
{
    int fd;
    struct dirent entries[8];
    int count;
    int index;
};

DIR *opendir(const char *pathname)
{
    int fd = open(pathname, O_RDONLY, 0);
    if (fd < 0) {
        return NULL;
    }

    struct file_stat stat;
    if (fstat(fd, &stat) < 0) {
        int saved_errno = errno;
        close(fd);
        errno = saved_errno;
        return NULL;
    }

    if (!S_ISDIR(stat.mode)) {
        close(fd);
        errno = ENOTDIR;
        return NULL;
    }

    DIR *dir = malloc(sizeof(DIR));
    if (dir == NULL) {
        close(fd);
        errno = ENOMEM;
        return NULL;
    }

    dir->fd = fd;
    dir->count = 0;
    dir->index = 0;
    memset(dir->entries, 0, sizeof(dir->entries));
    return dir;
}

struct dirent *readdir(DIR *dir)
{
    if (dir == NULL) {
        errno = EINVAL;
        return NULL;
    }

    if (dir->index >= dir->count) {
        errno = 0;
        int bytes = getdents(dir->fd, dir->entries, sizeof(dir->entries));
        if (bytes <= 0) {
            return NULL;
        }

        dir->count = bytes / (int)sizeof(struct dirent);
        dir->index = 0;
    }

    return &dir->entries[dir->index++];
}

int closedir(DIR *dir)
{
    if (dir == NULL) {
        errno = EINVAL;
        return -1;
    }

    int result = close(dir->fd);
    free(dir);
    return result;
}
