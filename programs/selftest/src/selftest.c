#include "errno.h"
#include "memory.h"
#include "polyos.h"
#include "stdio.h"
#include "stdlib.h"
#include "string.h"

static int passed = 0;
static int failed = 0;
static volatile int cow_global_value = 0x55667788;

static void ok(const char *name)
{
    passed++;
    printf("[ok] %s\n", name);
}

static void fail(const char *name, int code)
{
    failed++;
    printf("[fail] %s (%d)\n", name, code);
}

static void expect(const char *name, int condition, int code)
{
    if (condition) {
        ok(name);
    } else {
        fail(name, code);
    }
}

static void sleep_ms(u32 duration_ms)
{
    struct timespec req;
    req.tv_sec = duration_ms / 1000;
    req.tv_nsec = (duration_ms % 1000) * 1000000;
    nanosleep(&req, NULL);
}

static volatile int handled_signal = 0;

static void selftest_signal_handler(int signal)
{
    handled_signal = signal;
}

static int test_devices(void)
{
    int local_failed = failed;

    int fd = open("/dev/null", O_WRONLY, 0);
    expect("open /dev/null", fd >= 0, fd);
    if (fd >= 0) {
        const char msg[] = "discard me";
        expect("write /dev/null", write(fd, msg, sizeof(msg) - 1) == (ssize_t)(sizeof(msg) - 1), -1);
        expect("close /dev/null", close(fd) == 0, -1);
    }

    fd = open("/dev/null", O_WRONLY | O_APPEND, 0);
    expect("open /dev/null O_APPEND", fd >= 0, fd);
    if (fd >= 0) {
        const char msg[] = "append discard";
        expect("write /dev/null O_APPEND", write(fd, msg, sizeof(msg) - 1) == (ssize_t)(sizeof(msg) - 1), -1);
        expect("close /dev/null O_APPEND", close(fd) == 0, -1);
    }

    fd = open("/dev/null", O_WRONLY | O_TRUNC, 0);
    expect("open /dev/null O_TRUNC", fd >= 0, fd);
    if (fd >= 0) {
        const char msg[] = "truncate discard";
        expect("write /dev/null O_TRUNC", write(fd, msg, sizeof(msg) - 1) == (ssize_t)(sizeof(msg) - 1), -1);
        expect("close /dev/null O_TRUNC", close(fd) == 0, -1);
    }

    fd = open("/dev/zero", O_RDONLY, 0);
    expect("open /dev/zero", fd >= 0, fd);
    if (fd >= 0) {
        unsigned char zeros[8];
        memset(zeros, 0xff, sizeof(zeros));
        expect("read /dev/zero", read(fd, zeros, sizeof(zeros)) == (ssize_t)sizeof(zeros), -1);
        expect("content /dev/zero", memcmp(zeros, "\0\0\0\0\0\0\0\0", sizeof(zeros)) == 0, -1);
        expect("close /dev/zero", close(fd) == 0, -1);
    }

    struct winsize ws;
    memset(&ws, 0, sizeof(ws));
    int ioctl_result = ioctl(STDOUT_FILENO, TIOCGWINSZ, (unsigned long)&ws);
    expect("ioctl TIOCGWINSZ", ioctl_result == 0 && ws.ws_col > 0 && ws.ws_row > 0, ioctl_result);

    fd = open("/dev/tty", O_WRONLY, 0);
    expect("open /dev/tty", fd >= 0, fd);
    if (fd >= 0) {
        expect("write /dev/tty zero", write(fd, "", 0) == 0, -1);
        expect("close /dev/tty", close(fd) == 0, -1);
    }

    return failed == local_failed;
}

static int test_file_io(void)
{
    int local_failed = failed;
    const char path[] = "/tmp/selftest.txt";
    const char msg[] = "polyos selftest file";
    char buf[64];
    struct file_stat stat;

    int fd = open(path, O_CREAT | O_RDWR, 0);
    expect("open/create /tmp file", fd >= 0, fd);
    if (fd < 0) {
        return 0;
    }

    expect("write /tmp file", write(fd, msg, sizeof(msg) - 1) == (ssize_t)(sizeof(msg) - 1), -1);
    expect("lseek /tmp file", lseek(fd, 0, SEEK_SET) == 0, -1);

    memset(buf, 0, sizeof(buf));
    expect("read /tmp file", read(fd, buf, sizeof(msg) - 1) == (ssize_t)(sizeof(msg) - 1), -1);
    expect("content /tmp file", memcmp(buf, msg, sizeof(msg) - 1) == 0, -1);

    memset(&stat, 0, sizeof(stat));
    expect("fstat /tmp file", fstat(fd, &stat) == 0 && stat.size >= (int)(sizeof(msg) - 1), stat.size);
    expect("fstat regular mode", S_ISREG(stat.mode), stat.mode);
    expect("lseek SEEK_END", lseek(fd, 0, SEEK_END) == (off_t)(sizeof(msg) - 1), -1);
    expect("close /tmp file", close(fd) == 0, -1);

    fd = open(path, O_WRONLY | O_TRUNC, 0);
    expect("open O_TRUNC", fd >= 0, fd);
    if (fd >= 0) {
        expect("write after O_TRUNC", write(fd, "xy", 2) == 2, -1);
        expect("close O_TRUNC", close(fd) == 0, -1);
    }

    fd = open(path, O_WRONLY | O_APPEND, 0);
    expect("open O_APPEND", fd >= 0, fd);
    if (fd >= 0) {
        expect("write O_APPEND", write(fd, "z", 1) == 1, -1);
        expect("close O_APPEND", close(fd) == 0, -1);
    }

    fd = open(path, O_RDONLY, 0);
    expect("open after append", fd >= 0, fd);
    if (fd >= 0) {
        memset(buf, 0, sizeof(buf));
        expect("read after append", read(fd, buf, 8) == 3 && memcmp(buf, "xyz", 3) == 0, -1);
        expect("close after append", close(fd) == 0, -1);
    }

    return failed == local_failed;
}

static int test_unix_errno_dup_and_cwd(void)
{
    int local_failed = failed;
    char cwd[64];
    char buf[64];

    errno = 0;
    expect("errno open missing", open("/tmp/no-such-file", O_RDONLY, 0) == -1 && errno == ENOENT, errno);

    errno = 0;
    expect("errno create missing parent", open("/tmp/no-such-dir/file", O_CREAT | O_WRONLY, 0) == -1 && errno == ENOENT, errno);

    errno = 0;
    expect("errno bad fd", read(-1, buf, sizeof(buf)) == -1 && errno == EBADF, errno);

    int fd = open("/dev/zero", O_RDONLY, 0);
    expect("dup open /dev/zero", fd >= 0, fd);
    if (fd >= 0) {
        expect("fcntl F_GETFD", fcntl(fd, F_GETFD, 0) == 0, -1);
        expect("fcntl F_SETFD", fcntl(fd, F_SETFD, FD_CLOEXEC) == 0, -1);
        expect("fcntl FD_CLOEXEC", fcntl(fd, F_GETFD, 0) == FD_CLOEXEC, -1);
        expect("fcntl F_SETFL O_NONBLOCK", fcntl(fd, F_SETFL, O_NONBLOCK) == 0, -1);
        expect("fcntl F_GETFL O_NONBLOCK", (fcntl(fd, F_GETFL, 0) & O_NONBLOCK) == O_NONBLOCK, -1);

        int copy = dup(fd);
        expect("dup", copy >= 0 && copy != fd, copy);
        if (copy >= 0) {
            expect("dup clears cloexec", fcntl(copy, F_GETFD, 0) == 0, -1);
            memset(buf, 0xff, sizeof(buf));
            expect("dup read", read(copy, buf, 4) == 4 && memcmp(buf, "\0\0\0\0", 4) == 0, -1);
            close(copy);
        }

        int copy_min = fcntl(fd, F_DUPFD, 11);
        expect("fcntl F_DUPFD", copy_min >= 11, copy_min);
        if (copy_min >= 0) {
            close(copy_min);
        }

        int copy2 = dup2(fd, 10);
        expect("dup2", copy2 == 10, copy2);
        if (copy2 >= 0) {
            memset(buf, 0xff, sizeof(buf));
            expect("dup2 read", read(copy2, buf, 4) == 4 && memcmp(buf, "\0\0\0\0", 4) == 0, -1);
            close(copy2);
        }
        close(fd);
    }

    unlink("/tmp/selftest-dir/rel.txt");
    rmdir("/tmp/selftest-dir");
    expect("mkdir", mkdir("/tmp/selftest-dir", 0) == 0, -1);
    struct file_stat stat_buf;
    memset(&stat_buf, 0, sizeof(stat_buf));
    expect("stat directory", stat("/tmp/selftest-dir", &stat_buf) == 0 && S_ISDIR(stat_buf.mode), stat_buf.mode);
    expect("chdir", chdir("/tmp/selftest-dir") == 0, -1);
    memset(cwd, 0, sizeof(cwd));
    expect("getcwd", getcwd(cwd, sizeof(cwd)) == cwd && memcmp(cwd, "/tmp/selftest-dir", 17) == 0, -1);

    fd = open("rel.txt", O_CREAT | O_RDWR, 0);
    expect("open relative", fd >= 0, fd);
    if (fd >= 0) {
        const char msg[] = "relative";
        expect("write relative", write(fd, msg, sizeof(msg) - 1) == (ssize_t)(sizeof(msg) - 1), -1);
        expect("close relative", close(fd) == 0, -1);
    }

    memset(&stat_buf, 0, sizeof(stat_buf));
    expect("stat relative", stat("rel.txt", &stat_buf) == 0 && S_ISREG(stat_buf.mode), stat_buf.mode);
    memset(&stat_buf, 0, sizeof(stat_buf));
    expect("lstat relative", lstat("rel.txt", &stat_buf) == 0 && S_ISREG(stat_buf.mode), stat_buf.mode);
    errno = 0;
    expect("rmdir non-empty", rmdir("/tmp/selftest-dir") == -1 && errno == ENOTEMPTY, errno);

    int dirfd = open(".", O_RDONLY, 0);
    expect("open directory", dirfd >= 0, dirfd);
    if (dirfd >= 0) {
        struct dirent entries[8];
        memset(entries, 0, sizeof(entries));
        int bytes = getdents(dirfd, entries, sizeof(entries));
        int found = 0;
        if (bytes > 0) {
            int count = bytes / (int)sizeof(struct dirent);
            for (int i = 0; i < count; i++) {
                if (memcmp(entries[i].d_name, "rel.txt", 8) == 0) {
                    found = 1;
                    break;
                }
            }
        }
        expect("getdents", bytes > 0 && found, bytes);
        close(dirfd);
    }

    DIR *dir = opendir(".");
    expect("opendir", dir != NULL, errno);
    if (dir != NULL) {
        int found = 0;
        struct dirent *entry;
        while ((entry = readdir(dir)) != NULL) {
            if (memcmp(entry->d_name, "rel.txt", 8) == 0) {
                found = 1;
                break;
            }
        }
        expect("readdir", found, -1);
        expect("closedir", closedir(dir) == 0, -1);
    }

    expect("unlink relative", unlink("rel.txt") == 0, -1);
    expect("chdir root", chdir("/") == 0, -1);
    expect("rmdir", rmdir("/tmp/selftest-dir") == 0, -1);

    unlink("/tmp/selftest-mode.txt");
    int old_mask = umask(0027);
    expect("umask set", old_mask == 0022, old_mask);
    fd = open("/tmp/selftest-mode.txt", O_CREAT | O_RDWR | O_TRUNC, 0666);
    expect("open mode file", fd >= 0, fd);
    if (fd >= 0) {
        expect("close mode file", close(fd) == 0, -1);
    }
    memset(&stat_buf, 0, sizeof(stat_buf));
    expect("open applies umask", stat("/tmp/selftest-mode.txt", &stat_buf) == 0 && (stat_buf.mode & 0777) == 0640, stat_buf.mode);
    expect("chmod", chmod("/tmp/selftest-mode.txt", 0601) == 0, -1);
    memset(&stat_buf, 0, sizeof(stat_buf));
    expect("chmod mode", stat("/tmp/selftest-mode.txt", &stat_buf) == 0 && (stat_buf.mode & 0777) == 0601, stat_buf.mode);
    expect("chmod deny all", chmod("/tmp/selftest-mode.txt", 0000) == 0, -1);
    errno = 0;
    expect("permission read denied", open("/tmp/selftest-mode.txt", O_RDONLY, 0) == -1 && errno == EACCES, errno);
    errno = 0;
    expect("permission write denied", open("/tmp/selftest-mode.txt", O_WRONLY, 0) == -1 && errno == EACCES, errno);
    expect("chmod read only", chmod("/tmp/selftest-mode.txt", 0400) == 0, -1);
    fd = open("/tmp/selftest-mode.txt", O_RDONLY, 0);
    expect("permission read allowed", fd >= 0, fd);
    if (fd >= 0) {
        close(fd);
    }
    errno = 0;
    expect("permission write still denied", open("/tmp/selftest-mode.txt", O_WRONLY, 0) == -1 && errno == EACCES, errno);
    expect("chmod write only", chmod("/tmp/selftest-mode.txt", 0200) == 0, -1);
    fd = open("/tmp/selftest-mode.txt", O_WRONLY, 0);
    expect("permission write allowed", fd >= 0, fd);
    if (fd >= 0) {
        close(fd);
    }
    errno = 0;
    expect("permission read still denied", open("/tmp/selftest-mode.txt", O_RDONLY, 0) == -1 && errno == EACCES, errno);
    expect("chmod restore mode", chmod("/tmp/selftest-mode.txt", 0601) == 0, -1);
    expect("chown", chown("/tmp/selftest-mode.txt", 123, 456) == 0, -1);
    memset(&stat_buf, 0, sizeof(stat_buf));
    expect("chown ids", stat("/tmp/selftest-mode.txt", &stat_buf) == 0 && stat_buf.uid == 123 && stat_buf.gid == 456, stat_buf.uid);
    expect("chown keep uid", chown("/tmp/selftest-mode.txt", (unsigned int)-1, 789) == 0, -1);
    memset(&stat_buf, 0, sizeof(stat_buf));
    expect("chown kept uid", stat("/tmp/selftest-mode.txt", &stat_buf) == 0 && stat_buf.uid == 123 && stat_buf.gid == 789, stat_buf.gid);
    errno = 0;
    expect("chmod missing errno", chmod("/tmp/missing-mode-file", 0600) == -1 && errno == ENOENT, errno);
    const char noexec_path[] = "/tmp/selftest-noexec.elf";
    unlink(noexec_path);
    fd = open(noexec_path, O_CREAT | O_RDWR | O_TRUNC, 0644);
    expect("noexec create", fd >= 0, fd);
    if (fd >= 0) {
        close(fd);
        pid_t pid = fork();
        expect("noexec fork", pid >= 0, pid);
        if (pid == 0) {
            char *exec_args[] = { (char *)noexec_path, NULL };
            errno = 0;
            execve(noexec_path, exec_args, NULL);
            _exit(errno == EACCES ? 0 : errno);
        }
        if (pid > 0) {
            int status = -1;
            expect("execute denied", waitpid(pid, &status, 0) == pid && WIFEXITED(status) && WEXITSTATUS(status) == 0, status);
        }
        unlink(noexec_path);
    }
    expect("umask restore", umask(old_mask) == 0027, -1);
    expect("unlink mode file", unlink("/tmp/selftest-mode.txt") == 0, -1);

    return failed == local_failed;
}

static int test_pipe(void)
{
    int local_failed = failed;
    int fds[2];
    const char msg[] = "pipe-ok";
    char buf[64];

    expect("pipe create", pipe(fds) == 0, -1);
    if (failed != local_failed) {
        return 0;
    }

    expect("pipe write", write(fds[1], msg, sizeof(msg) - 1) == (ssize_t)(sizeof(msg) - 1), -1);
    memset(buf, 0, sizeof(buf));
    expect("pipe read", read(fds[0], buf, sizeof(msg) - 1) == (ssize_t)(sizeof(msg) - 1), -1);
    expect("pipe content", memcmp(buf, msg, sizeof(msg) - 1) == 0, -1);
    expect("pipe close read", close(fds[0]) == 0, -1);
    expect("pipe close write", close(fds[1]) == 0, -1);

    expect("pipe nonblock create", pipe(fds) == 0, -1);
    if (failed == local_failed) {
        expect("pipe nonblock set", fcntl(fds[0], F_SETFL, O_NONBLOCK) == 0, -1);
        errno = 0;
        expect("pipe nonblock empty read", read(fds[0], buf, 1) == -1 && errno == EAGAIN, errno);
        close(fds[0]);
        close(fds[1]);
    }

    const char blocking_msg[] = "pipe-blocking-read";
    expect("pipe blocking read create", pipe(fds) == 0, -1);
    if (failed == local_failed) {
        pid_t pid = fork();
        expect("pipe blocking read fork", pid >= 0, pid);
        if (pid == 0) {
            close(fds[0]);
            sleep_ms(10);
            write(fds[1], blocking_msg, sizeof(blocking_msg) - 1);
            close(fds[1]);
            _exit(0);
        }
        if (pid >= 0) {
            close(fds[1]);
            memset(buf, 0, sizeof(buf));
            expect("pipe blocking read", read(fds[0], buf, sizeof(blocking_msg) - 1) == (ssize_t)(sizeof(blocking_msg) - 1), -1);
            expect("pipe blocking read content", memcmp(buf, blocking_msg, sizeof(blocking_msg) - 1) == 0, -1);
            close(fds[0]);
            int status = -1;
            expect("pipe blocking read wait", waitpid(pid, &status, 0) == pid && WIFEXITED(status), status);
        }
    }

    expect("pipe blocking write create", pipe(fds) == 0, -1);
    if (failed == local_failed) {
        char fill[4096];
        memset(fill, 'x', sizeof(fill));
        expect("pipe fill", write(fds[1], fill, sizeof(fill)) == (ssize_t)sizeof(fill), -1);
        pid_t pid = fork();
        expect("pipe blocking write fork", pid >= 0, pid);
        if (pid == 0) {
            char one = 0;
            sleep_ms(10);
            read(fds[0], &one, 1);
            close(fds[0]);
            close(fds[1]);
            _exit(0);
        }
        if (pid >= 0) {
            const char one = 'y';
            expect("pipe blocking write", write(fds[1], &one, 1) == 1, -1);
            int status = -1;
            expect("pipe blocking write wait", waitpid(pid, &status, 0) == pid && WIFEXITED(status), status);
            close(fds[0]);
            close(fds[1]);
        }
    }

    return failed == local_failed;
}

static int test_semaphore_basic(void)
{
    int local_failed = failed;
    sem_t sem = -1;
    expect("sem_init", sem_init(&sem, 0, 1) == 0 && sem > 0, sem);
    if (sem <= 0) {
        return 0;
    }

    expect("sem_wait immediate", sem_wait(&sem) == 0, -1);
    expect("sem_post", sem_post(&sem) == 0, -1);
    expect("sem_destroy", sem_destroy(&sem) == 0, -1);
    errno = 0;
    expect("sem_init NULL errno", sem_init(NULL, 0, 1) == -1 && errno == EFAULT, errno);
    errno = 0;
    expect("sem_init pshared errno", sem_init(&sem, 1, 1) == -1 && errno == ENOTSUP, errno);
    errno = 0;
    expect("sem_wait NULL errno", sem_wait(NULL) == -1 && errno == EFAULT, errno);
    errno = 0;
    expect("sem_post NULL errno", sem_post(NULL) == -1 && errno == EFAULT, errno);
    errno = 0;
    expect("sem_destroy NULL errno", sem_destroy(NULL) == -1 && errno == EFAULT, errno);

    sem = 999999;
    errno = 0;
    expect("sem_wait invalid errno", sem_wait(&sem) == -1 && errno == EINVAL, errno);
    errno = 0;
    expect("sem_post invalid errno", sem_post(&sem) == -1 && errno == EINVAL, errno);
    errno = 0;
    expect("sem_destroy invalid errno", sem_destroy(&sem) == -1 && errno == EINVAL, errno);

    return failed == local_failed;
}

static int test_socket_errno(void)
{
    int local_failed = failed;
    char buf[8];
    int one = 1;
    struct sockaddr_in addr;
    struct sockaddr_in out_addr;
    socklen_t addr_len;

    errno = 0;
    expect("socket invalid domain errno", socket(99, SOCK_RAW, IPPROTO_ICMP) == -1 && errno == ENOTSUP, errno);

    int fd = socket(AF_INET, SOCK_RAW, IPPROTO_ICMP);
    expect("socket raw icmp", fd >= 0, fd);
    if (fd < 0) {
        return 0;
    }

    errno = 0;
    expect("recvfrom empty errno", recvfrom(fd, buf, sizeof(buf), 0, NULL, NULL) == -1 && errno == EAGAIN, errno);
    errno = 0;
    expect("recv empty errno", recv(fd, buf, sizeof(buf), 0) == -1 && errno == EAGAIN, errno);
    errno = 0;
    expect("sendto null dest errno", sendto(fd, buf, sizeof(buf), 0, NULL, 0) == -1 && errno == EFAULT, errno);
    errno = 0;
    expect("send unconnected errno", send(fd, buf, sizeof(buf), 0) == -1 && errno == ENOTCONN, errno);
    expect("setsockopt SO_REUSEADDR", setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &one, sizeof(one)) == 0, -1);

    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(0);
    addr.sin_addr.s_addr = 0;
    expect("bind raw socket", bind(fd, (struct sockaddr *)&addr, sizeof(addr)) == 0, -1);

    memset(&out_addr, 0, sizeof(out_addr));
    addr_len = sizeof(out_addr);
    expect("getsockname", getsockname(fd, (struct sockaddr *)&out_addr, &addr_len) == 0 && out_addr.sin_family == AF_INET, errno);

    memset(&out_addr, 0, sizeof(out_addr));
    addr_len = sizeof(out_addr);
    errno = 0;
    expect("getpeername unconnected errno", getpeername(fd, (struct sockaddr *)&out_addr, &addr_len) == -1 && errno == ENOTCONN, errno);

    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(0);
    addr.sin_addr.s_addr = (192u << 24) | (0u << 16) | (2u << 8) | 1u;
    expect("connect raw socket", connect(fd, (struct sockaddr *)&addr, sizeof(addr)) == 0, -1);

    memset(&out_addr, 0, sizeof(out_addr));
    addr_len = sizeof(out_addr);
    expect("getpeername", getpeername(fd, (struct sockaddr *)&out_addr, &addr_len) == 0 && out_addr.sin_addr.s_addr == addr.sin_addr.s_addr, errno);

    errno = 0;
    expect("listen unsupported errno", listen(fd, 1) == -1 && errno == ENOTSUP, errno);
    errno = 0;
    expect("accept unsupported errno", accept(fd, NULL, NULL) == -1 && errno == ENOTSUP, errno);
    expect("socket close", close(fd) == 0, -1);

    errno = 0;
    expect("recvfrom bad fd errno", recvfrom(fd, buf, sizeof(buf), 0, NULL, NULL) == -1 && errno == EBADF, errno);

    return failed == local_failed;
}

static int test_fork_pipe_semaphore(void)
{
    int local_failed = failed;
    int fds[2];
    sem_t sem = -1;
    volatile int cow_value = 0x11112222;
    const char msg[] = "fork-sync-ok";
    char buf[32];
    char *cow_heap = malloc(32);

    expect("fork cow heap alloc", cow_heap != NULL, errno);
    if (cow_heap == NULL) {
        return 0;
    }
    memset(cow_heap, 0x5a, 32);
    cow_global_value = 0x55667788;

    expect("fork sem_init", sem_init(&sem, 0, 0) == 0 && sem > 0, sem);
    expect("fork pipe create", pipe(fds) == 0, -1);
    if (sem <= 0 || failed != local_failed) {
        free(cow_heap);
        return 0;
    }

    pid_t pid = fork();
    expect("fork", pid >= 0, pid);

    if (pid == 0) {
        close(fds[0]);
        cow_value = 0x33334444;
        cow_global_value = 0x11223344;
        cow_heap[0] = 0x11;
        if (write(fds[1], msg, sizeof(msg) - 1) != (ssize_t)(sizeof(msg) - 1)) {
            _exit(31);
        }
        close(fds[1]);
        if (sem_post(&sem) != 0) {
            _exit(32);
        }
        _exit(23);
    }

    if (pid < 0) {
        return 0;
    }

    close(fds[1]);
    expect("sem_wait blocks until child signal", sem_wait(&sem) == 0, -1);

    memset(buf, 0, sizeof(buf));
    expect("fork pipe read", read(fds[0], buf, sizeof(msg) - 1) == (ssize_t)(sizeof(msg) - 1), -1);
    expect("fork pipe content", memcmp(buf, msg, sizeof(msg) - 1) == 0, -1);
    close(fds[0]);

    int status = -1;
    pid_t waited = waitpid(pid, &status, 0);
    expect("waitpid child", waited == pid && WIFEXITED(status) && WEXITSTATUS(status) == 23, status);
    expect("fork cow stack", cow_value == 0x11112222, cow_value);
    expect("fork cow global", cow_global_value == 0x55667788, cow_global_value);
    expect("fork cow heap", cow_heap[0] == 0x5a, cow_heap[0]);
    free(cow_heap);
    expect("fork sem_destroy", sem_destroy(&sem) == 0, -1);

    return failed == local_failed;
}

static int test_fork_fd_state(void)
{
    int local_failed = failed;
    const char path[] = "/tmp/fork-fd-state.txt";
    const char data[] = "abcdef";
    char buf[4];

    unlink(path);
    int fd = open(path, O_CREAT | O_RDWR | O_TRUNC, 0);
    expect("fork fd open", fd >= 0, fd);
    if (fd < 0) {
        return 0;
    }

    expect("fork fd write", write(fd, data, sizeof(data) - 1) == (ssize_t)(sizeof(data) - 1), -1);
    expect("fork fd seek", lseek(fd, 0, SEEK_SET) == 0, -1);

    pid_t pid = fork();
    expect("fork fd fork", pid >= 0, pid);
    if (pid == 0) {
        char child_buf[2];
        ssize_t read_bytes = read(fd, child_buf, sizeof(child_buf));
        _exit(read_bytes == (ssize_t)sizeof(child_buf) && memcmp(child_buf, "ab", 2) == 0 ? 0 : 52);
    }

    if (pid < 0) {
        close(fd);
        unlink(path);
        return 0;
    }

    int status = -1;
    pid_t waited = waitpid(pid, &status, 0);
    expect("fork fd child", waited == pid && WIFEXITED(status) && WEXITSTATUS(status) == 0, status);

    memset(buf, 0, sizeof(buf));
    expect("fork fd shared offset", read(fd, buf, 2) == 2 && memcmp(buf, "cd", 2) == 0, -1);
    expect("fork fd close", close(fd) == 0, -1);
    expect("fork fd unlink", unlink(path) == 0, -1);

    return failed == local_failed;
}

static int test_waitpid_zombie_reparent(void)
{
    int local_failed = failed;
    int status = -1;

    pid_t pid = fork();
    expect("waitpid WNOHANG fork", pid >= 0, pid);
    if (pid == 0) {
        sleep_ms(10);
        _exit(17);
    }
    if (pid < 0) {
        return 0;
    }

    expect("waitpid WNOHANG running", waitpid(pid, &status, WNOHANG) == 0, status);
    pid_t waited = waitpid(pid, &status, 0);
    expect("waitpid after WNOHANG", waited == pid && WIFEXITED(status) && WEXITSTATUS(status) == 17, status);

    pid = fork();
    expect("waitpid zombie fork", pid >= 0, pid);
    if (pid == 0) {
        _exit(18);
    }
    if (pid < 0) {
        return 0;
    }

    sleep_ms(10);
    status = -1;
    waited = waitpid(pid, &status, WNOHANG);
    expect("waitpid zombie retained", waited == pid && WIFEXITED(status) && WEXITSTATUS(status) == 18, status);
    errno = 0;
    expect("waitpid reaped child missing", waitpid(pid, &status, WNOHANG) == -1 && errno == ECHILD, errno);

    int fds[2];
    expect("orphan pipe create", pipe(fds) == 0, -1);
    if (failed != local_failed) {
        return 0;
    }

    pid = fork();
    expect("orphan parent fork", pid >= 0, pid);
    if (pid == 0) {
        close(fds[0]);
        pid_t grandchild = fork();
        if (grandchild == 0) {
            sleep_ms(20);
            pid_t parent = getppid();
            write(fds[1], &parent, sizeof(parent));
            close(fds[1]);
            _exit(parent == 0 ? 0 : 1);
        }
        close(fds[1]);
        _exit(grandchild < 0 ? 19 : 0);
    }
    if (pid < 0) {
        close(fds[0]);
        close(fds[1]);
        return 0;
    }

    close(fds[1]);
    status = -1;
    waited = waitpid(pid, &status, 0);
    expect("orphan parent waited", waited == pid && WIFEXITED(status) && WEXITSTATUS(status) == 0, status);

    pid_t reparented_to = -1;
    for (int i = 0; i < 100; i++) {
        ssize_t read_bytes = read(fds[0], &reparented_to, sizeof(reparented_to));
        if (read_bytes == (ssize_t)sizeof(reparented_to)) {
            break;
        }
        sleep_ms(1);
    }
    close(fds[0]);
    expect("orphan reparent to init", reparented_to == 0, reparented_to);

    return failed == local_failed;
}

static int test_signals(void)
{
    int local_failed = failed;
    struct sigaction act;
    struct sigaction oldact;
    int status = -1;

    errno = 0;
    expect("kill self signal 0", kill(getpid(), 0) == 0, errno);
    errno = 0;
    expect("kill missing pid errno", kill(999999, 0) == -1 && errno == ESRCH, errno);
    errno = 0;
    expect("kill invalid signal errno", kill(getpid(), 99) == -1 && errno == EINVAL, errno);

    memset(&act, 0, sizeof(act));
    memset(&oldact, 0, sizeof(oldact));
    act.sa_handler = SIG_IGN;
    expect("sigaction ignore", sigaction(SIGUSR1, &act, &oldact) == 0 && oldact.sa_handler == SIG_DFL, errno);
    expect("kill ignored signal", kill(getpid(), SIGUSR1) == 0, errno);
    memset(&oldact, 0, sizeof(oldact));
    expect("sigaction readback", sigaction(SIGUSR1, NULL, &oldact) == 0 && oldact.sa_handler == SIG_IGN, errno);

    memset(&act, 0, sizeof(act));
    act.sa_handler = SIG_DFL;
    expect("sigaction reset", sigaction(SIGUSR1, &act, NULL) == 0, errno);

    handled_signal = 0;
    memset(&act, 0, sizeof(act));
    act.sa_handler = selftest_signal_handler;
    expect("sigaction handler", sigaction(SIGUSR2, &act, NULL) == 0, errno);
    expect("kill handled signal", kill(getpid(), SIGUSR2) == 0 && handled_signal == SIGUSR2, handled_signal);

    memset(&act, 0, sizeof(act));
    act.sa_handler = SIG_DFL;
    expect("sigaction handler reset", sigaction(SIGUSR2, &act, NULL) == 0, errno);

    memset(&act, 0, sizeof(act));
    act.sa_handler = SIG_IGN;
    errno = 0;
    expect("sigaction SIGKILL invalid", sigaction(SIGKILL, &act, NULL) == -1 && errno == EINVAL, errno);

    pid_t pid = fork();
    expect("signal fork", pid >= 0, pid);
    if (pid == 0) {
        sleep_ms(1000);
        _exit(45);
    }
    if (pid < 0) {
        return 0;
    }

    sleep_ms(5);
    expect("kill child SIGTERM", kill(pid, SIGTERM) == 0, errno);
    pid_t waited = waitpid(pid, &status, 0);
    expect("waitpid signaled child", waited == pid && WIFSIGNALED(status) && WTERMSIG(status) == SIGTERM, status);

    handled_signal = 0;
    memset(&act, 0, sizeof(act));
    act.sa_handler = selftest_signal_handler;
    expect("sigaction SIGCHLD handler", sigaction(SIGCHLD, &act, NULL) == 0, errno);

    pid = fork();
    expect("SIGCHLD fork", pid >= 0, pid);
    if (pid == 0) {
        _exit(0);
    }
    if (pid >= 0) {
        status = -1;
        waited = waitpid(pid, &status, 0);
        expect("waitpid SIGCHLD child", waited == pid && WIFEXITED(status) && WEXITSTATUS(status) == 0, status);
        expect("SIGCHLD handler ran", handled_signal == SIGCHLD, handled_signal);
    }

    memset(&act, 0, sizeof(act));
    act.sa_handler = SIG_DFL;
    expect("sigaction SIGCHLD reset", sigaction(SIGCHLD, &act, NULL) == 0, errno);

    return failed == local_failed;
}

static int test_memory_and_sleep(void)
{
    int local_failed = failed;
    void *brk_before_malloc = NULL;

    void *heap = sbrk(64);
    expect("sbrk", heap != (void *)-1, errno);
    if (heap != (void *)-1) {
        memset(heap, 0xcd, 64);
        ok("sbrk write");
        brk_before_malloc = sbrk(0);
    }

    void *ptr = malloc(128);
    expect("malloc", ptr != NULL, 0);
    if (ptr != NULL && brk_before_malloc != (void *)-1 && brk_before_malloc != NULL) {
        void *brk_after_malloc = sbrk(0);
        memset(ptr, 0xab, 128);
        free(ptr);
        ok("free");

        void *brk_after_free = sbrk(0);
        expect("malloc grows brk", brk_after_malloc >= brk_before_malloc, 0);
        expect("free releases tail block", brk_after_free == brk_before_malloc, 0);

        void *ptr_reuse = malloc(128);
        expect("malloc reuse", ptr_reuse != NULL, 0);
        if (ptr_reuse != NULL) {
            free(ptr_reuse);
            ok("free reuse");
        }
    } else if (ptr != NULL) {
        memset(ptr, 0xab, 128);
        free(ptr);
        ok("free");
    }

    sleep_ms(1);
    ok("sleep");

    expect("getpid", getpid() >= 0, -1);
    expect("getppid", getppid() >= 0, -1);
    expect("getuid", getuid() == 0, getuid());
    expect("getgid", getgid() == 0, getgid());
    expect("geteuid", geteuid() == 0, geteuid());
    expect("getegid", getegid() == 0, getegid());
    errno = 0;
    expect("reboot invalid cmd errno", reboot(0) == -1 && errno == EINVAL, errno);

    return failed == local_failed;
}

static u32 timespec_to_ms(const struct timespec *ts)
{
    return (u32)ts->tv_sec * 1000 + (u32)ts->tv_nsec / 1000000;
}

static int test_time_syscalls(void)
{
    int local_failed = failed;
    struct timespec before;
    struct timespec after;
    struct timeval tv;

    memset(&before, 0, sizeof(before));
    memset(&after, 0, sizeof(after));
    memset(&tv, 0, sizeof(tv));

    expect("clock_gettime monotonic", clock_gettime(CLOCK_MONOTONIC, &before) == 0, errno);
    expect("gettimeofday", gettimeofday(&tv, NULL) == 0 && tv.tv_sec >= 0 && tv.tv_usec >= 0, errno);

    struct timespec req;
    req.tv_sec = 0;
    req.tv_nsec = 1000000;
    expect("nanosleep", nanosleep(&req, NULL) == 0, errno);
    expect("clock_gettime after sleep", clock_gettime(CLOCK_MONOTONIC, &after) == 0 && timespec_to_ms(&after) >= timespec_to_ms(&before), errno);

    req.tv_sec = 0;
    req.tv_nsec = 1000000000;
    errno = 0;
    expect("nanosleep invalid nsec", nanosleep(&req, NULL) == -1 && errno == EINVAL, errno);

    errno = 0;
    expect("clock_gettime invalid clock", clock_gettime(99, &after) == -1 && errno == EINVAL, errno);
    expect("sleep zero", sleep(0) == 0, -1);

    return failed == local_failed;
}

static int wait_for_dhcp_bound(struct network_info *info)
{
    for (int i = 0; i < 5000; i++) {
        if (network_info(info) == 0 && info->present && info->dhcp_state == 3) {
            return 1;
        }
        sleep_ms(1);
    }

    return 0;
}

static int test_environment(void)
{
    int local_failed = failed;

    char *path = getenv("PATH");
    expect("getenv PATH", path && strncmp(path, "/bin", 5) == 0, path ? 0 : -1);
    expect("setenv", setenv("SELFTEST_ENV", "ok", 1) == 0, -1);
    char *value = getenv("SELFTEST_ENV");
    expect("getenv after setenv", value && strncmp(value, "ok", 3) == 0, value ? 0 : -1);
    expect("unsetenv", unsetenv("SELFTEST_ENV") == 0 && getenv("SELFTEST_ENV") == NULL, -1);

    return failed == local_failed;
}

static int test_network(void)
{
    int local_failed = failed;
    struct network_info info;
    memset(&info, 0, sizeof(info));

    expect("network_info", network_info(&info) == 0 && info.present, -1);
    if (!info.present) {
        return 0;
    }

    if (info.dhcp_state != 3) {
        expect("network_dhcp_discover", network_dhcp_discover() == 0, -1);
    }

    expect("dhcp bound", wait_for_dhcp_bound(&info), info.dhcp_state);

    if (info.dhcp_state == 3) {
        u32 ping_rx_before = info.ping_rx;
        expect("network_ping_gateway", network_ping_gateway() == 0, -1);
        for (int i = 0; i < 2000; i++) {
            network_info(&info);
            if (info.ping_rx > ping_rx_before) {
                break;
            }
            sleep_ms(1);
        }
        expect("gateway ping reply", info.ping_rx > ping_rx_before, info.ping_rx);
    }

    return failed == local_failed;
}

static int wants_network(int argc, char **argv)
{
    return argc > 1 && strncmp(argv[1], "net", 4) == 0;
}

int main(int argc, char **argv)
{
    printf("selftest: start\n");

    expect("kernel selftest", kernel_selftest() == 0, -1);
    test_memory_and_sleep();
    test_time_syscalls();
    test_devices();
    test_file_io();
    test_unix_errno_dup_and_cwd();
    test_pipe();
    test_semaphore_basic();
    test_socket_errno();
    test_fork_pipe_semaphore();
    test_fork_fd_state();
    test_waitpid_zombie_reparent();
    test_signals();
    test_environment();

    if (wants_network(argc, argv)) {
        test_network();
    } else {
        printf("[skip] network (run /bin/selftest.elf net)\n");
    }

    printf("selftest: passed=%d failed=%d\n", passed, failed);
    return failed == 0 ? 0 : 1;
}
