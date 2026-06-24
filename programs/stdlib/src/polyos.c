#include "polyos.h"
#include "stdlib.h"
#include "string.h"
#include "stdio.h"

static void sleep_ms(u32 duration_ms)
{
    struct timespec req;
    req.tv_sec = duration_ms / 1000;
    req.tv_nsec = (duration_ms % 1000) * 1000000;
    nanosleep(&req, NULL);
}

void polyos_terminal_readline(char* out, int max, bool output_while_typing)
{
    int i = 0;
    while (i < max - 1) {
        char key = 0;
        if (read(STDIN_FILENO, &key, 1) != 1){
            sleep_ms(1);
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

static int contains_slash(const char *value){
    for (int i = 0; value[i]; i++){
        if (value[i] == '/'){
            return 1;
        }
    }
    return 0;
}

static int build_exec_path(char *out, int out_size, const char *prefix, const char *command, const char *suffix){
    int pos = 0;

    for (int i = 0; prefix[i]; i++){
        if (pos + 1 >= out_size){
            return 0;
        }
        out[pos++] = prefix[i];
    }

    for (int i = 0; command[i]; i++){
        if (pos + 1 >= out_size){
            return 0;
        }
        out[pos++] = command[i];
    }

    for (int i = 0; suffix[i]; i++){
        if (pos + 1 >= out_size){
            return 0;
        }
        out[pos++] = suffix[i];
    }

    out[pos] = '\0';
    return 1;
}

static void exec_if_present(const char *path, char *const argv[]){
    struct file_stat stat_buf;
    if (stat(path, &stat_buf) == 0 && !S_ISDIR(stat_buf.mode)){
        execve(path, argv, environ);
    }
}

static void exec_from_path(const char *command, char *const argv[]){
    const char *path_env = getenv("PATH");
    if (!path_env || !path_env[0]){
        path_env = "/bin";
    }

    char directory[256];
    char prefix_buf[256];
    char path[256];
    int dir_len = 0;

    for (int i = 0;; i++){
        char c = path_env[i];
        if (c != ':' && c != '\0'){
            if (dir_len + 1 < (int)sizeof(directory)){
                directory[dir_len++] = c;
            }
            continue;
        }

        directory[dir_len] = '\0';
        const char *prefix = dir_len == 0 ? "." : directory;
        if (!build_exec_path(prefix_buf, sizeof(prefix_buf), prefix, "/", "")){
            if (c == '\0'){
                break;
            }
            dir_len = 0;
            continue;
        }

        if (build_exec_path(path, sizeof(path), prefix_buf, command, "")){
            exec_if_present(path, argv);
        }
        if (build_exec_path(path, sizeof(path), prefix_buf, command, ".elf")){
            exec_if_present(path, argv);
        }

        if (c == '\0'){
            break;
        }
        dir_len = 0;
    }
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
        if (contains_slash(argv[0])){
            exec_if_present(argv[0], argv);
        } else {
            exec_from_path(argv[0], argv);
        }
        _exit(127);
    }

    int status = 0;
    if (waitpid(pid, &status, 0) < 0){
        return -1;
    }

    if (WIFEXITED(status)){
        return WEXITSTATUS(status);
    }

    return -1;
}
