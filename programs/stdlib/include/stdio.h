#ifndef POLYOS_STDIO_H
#define POLYOS_STDIO_H

#include <types.h>

typedef unsigned int FILE_SEEK_MODE;
enum
{
    FILE_SEEK_SET,
    FILE_SEEK_CUR,
    FILE_SEEK_END
};

#define SEEK_SET FILE_SEEK_SET
#define SEEK_CUR FILE_SEEK_CUR
#define SEEK_END FILE_SEEK_END

#define O_RDONLY 0
#define O_WRONLY 1
#define O_RDWR 2
#define O_CREAT 0x40
#define O_TRUNC 0x200
#define O_APPEND 0x400
#define O_NONBLOCK 0x800

#define STDIN_FILENO 0
#define STDOUT_FILENO 1
#define STDERR_FILENO 2

#define TIOCGWINSZ 0x5413
#define POLYOS_IOCTL_SCREEN_CLEAR 0x5001
#define POLYOS_IOCTL_SCREEN_SET_COLOR 0x5002
#define POLYOS_IOCTL_SCREEN_DISABLE_CURSOR 0x5003
#define POLYOS_VGA_COLOR(foreground, background) ((((background) & 0x0f) << 4) | ((foreground) & 0x0f))

enum
{
    POLYOS_COLOR_BLACK = 0,
    POLYOS_COLOR_BLUE = 1,
    POLYOS_COLOR_GREEN = 2,
    POLYOS_COLOR_CYAN = 3,
    POLYOS_COLOR_RED = 4,
    POLYOS_COLOR_MAGENTA = 5,
    POLYOS_COLOR_BROWN = 6,
    POLYOS_COLOR_LIGHT_GRAY = 7,
    POLYOS_COLOR_DARK_GRAY = 8,
    POLYOS_COLOR_LIGHT_BLUE = 9,
    POLYOS_COLOR_LIGHT_GREEN = 10,
    POLYOS_COLOR_LIGHT_CYAN = 11,
    POLYOS_COLOR_LIGHT_RED = 12,
    POLYOS_COLOR_PINK = 13,
    POLYOS_COLOR_YELLOW = 14,
    POLYOS_COLOR_WHITE = 15,
};

struct winsize
{
    u16 ws_row;
    u16 ws_col;
    u16 ws_xpixel;
    u16 ws_ypixel;
};

typedef struct DIR DIR;

int putchar(int c);
int printf(const char *fmt, ...);
int serial_printf(const char *fmt, ...);

int open(const char *pathname, int flags, int mode);
ssize_t read(int fd, void *buf, size_t count);
ssize_t write(int fd, const void *buf, size_t count);
off_t lseek(int fd, off_t offset, int whence);
int stat(const char *pathname, struct file_stat *stat);
int lstat(const char *pathname, struct file_stat *stat);
int fstat(int fd, struct file_stat *stat);
int ioctl(int fd, unsigned long request, unsigned long arg);
int fcntl(int fd, int cmd, long arg);
int close(int fd);
int pipe(int pipefd[2]);
int dup(int oldfd);
int dup2(int oldfd, int newfd);
int unlink(const char *pathname);
int mkdir(const char *pathname, int mode);
int rmdir(const char *pathname);
int chdir(const char *pathname);
char *getcwd(char *buf, size_t size);
int getdents(int fd, struct dirent *dirp, size_t count);
DIR *opendir(const char *pathname);
struct dirent *readdir(DIR *dir);
int closedir(DIR *dir);

int fopen(const char *filename, const char *mode);
int fread(int fd, void *ptr, int size);
int fwrite(int fd, void *ptr, int size);
int fseek(int fd, int offset, FILE_SEEK_MODE mode);
int fclose(int fd);

#endif
