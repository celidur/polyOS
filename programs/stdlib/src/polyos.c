#include "polyos.h"
#include "string.h"
#include "stdio.h"

#define POLYOS_WAIT_TIMEOUT_CODE -2

int recvfrom_wait(int sockfd, void *buf, size_t len, int flags, struct sockaddr *src_addr, socklen_t *addrlen, u32 timeout_ticks){
    u32 attempts = 0;
    while (timeout_ticks == 0 || attempts < timeout_ticks) {
        int res = recvfrom(sockfd, buf, len, flags, src_addr, addrlen);
        if (res >= 0) {
            return res;
        }
        polyos_sleep(1);
        attempts++;
    }

    return POLYOS_WAIT_TIMEOUT_CODE;
}

void polyos_terminal_readline(char* out, int max, bool output_while_typing)
{
    int i = 0;
    while (i < max - 1) {
        char key = 0;
        if (read(STDIN_FILENO, &key, 1) != 1){
            polyos_sleep(1);
            continue;
        }

        // Carriage return means we're done
        if (key == 13){
            break;
        }

        if (output_while_typing && key != 0x08){
            write(STDOUT_FILENO, &key, 1);
        }

        // Backspace
        if (key == 0x08){
            if (i > 0) {
                write(STDOUT_FILENO, &key, 1);
                i -= 1;
                out[i] = '\0';
            }
            continue;
        }

        out[i] = key;
        i++;
    }
    // Null terminate
    out[i] = '\0';
}

int clear_screen()
{
    return ioctl(STDOUT_FILENO, POLYOS_IOCTL_SCREEN_CLEAR, 0);
}

int fopen(const char *filename, const char *mode){
    int flags = O_RDONLY;

    if (mode && mode[0] == 'w'){
        flags = O_WRONLY | O_CREAT | O_TRUNC;
    } else if (mode && mode[0] == 'a'){
        flags = O_WRONLY | O_CREAT | O_APPEND;
    }

    if (mode){
        for (int i = 0; mode[i]; i++){
            if (mode[i] == '+'){
                flags = (flags & ~(O_RDONLY | O_WRONLY)) | O_RDWR;
                break;
            }
        }
    }

    return open(filename, flags, 0);
}

int fread(int fd, void *ptr, int size){
    return read(fd, ptr, size);
}

int fwrite(int fd, void *ptr, int size){
    return write(fd, ptr, size);
}

int fseek(int fd, int offset, FILE_SEEK_MODE mode){
    return lseek(fd, offset, mode) < 0 ? -1 : 0;
}

int fclose(int fd){
    return close(fd);
}

int polyos_system_run(const char *command){
    char buff[1024];
    char *argv[64];
    int argc = 0;

    strncpy(buff, command, sizeof(buff));
    buff[sizeof(buff) - 1] = '\0';

    char* token = strtok(buff, " ");
    while(token && argc < (int)(sizeof(argv) / sizeof(argv[0])) - 1){
        argv[argc++] = token;
        token = strtok(NULL, " ");
    }
    argv[argc] = NULL;

    if (argc == 0){
        return -1;
    }

    pid_t pid = fork();
    if (pid < 0){
        return -1;
    }

    if (pid == 0){
        execve(argv[0], argv, NULL);
        _exit(127);
    }

    int status = 0;
    if (waitpid(pid, &status, 0) < 0){
        return -1;
    }

    return status;
}
