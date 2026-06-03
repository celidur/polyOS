#include "memory.h"
#include "polyos.h"
#include "stdio.h"
#include "stdlib.h"
#include "string.h"

static int passed = 0;
static int failed = 0;

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
    expect("close /tmp file", close(fd) == 0, -1);

    return failed == local_failed;
}

static int test_pipe(void)
{
    int local_failed = failed;
    int fds[2];
    const char msg[] = "pipe-ok";
    char buf[16];

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

    return failed == local_failed;
}

static int test_semaphore_basic(void)
{
    int local_failed = failed;
    int sem = sem_create(1);
    expect("sem_create", sem > 0, sem);
    if (sem <= 0) {
        return 0;
    }

    expect("sem_wait immediate", sem_wait(sem) == 0, -1);
    expect("sem_signal", sem_signal(sem) == 0, -1);
    expect("sem_close", sem_close(sem) == 0, -1);

    return failed == local_failed;
}

static int test_fork_pipe_semaphore(void)
{
    int local_failed = failed;
    int fds[2];
    int sem = sem_create(0);
    volatile int cow_value = 0x11112222;
    const char msg[] = "fork-sync-ok";
    char buf[32];

    expect("fork sem_create", sem > 0, sem);
    expect("fork pipe create", pipe(fds) == 0, -1);
    if (sem <= 0 || failed != local_failed) {
        return 0;
    }

    pid_t pid = fork();
    expect("fork", pid >= 0, pid);

    if (pid == 0) {
        close(fds[0]);
        cow_value = 0x33334444;
        if (write(fds[1], msg, sizeof(msg) - 1) != (ssize_t)(sizeof(msg) - 1)) {
            _exit(31);
        }
        close(fds[1]);
        if (sem_signal(sem) != 0) {
            _exit(32);
        }
        _exit(23);
    }

    if (pid < 0) {
        return 0;
    }

    close(fds[1]);
    expect("sem_wait blocks until child signal", sem_wait(sem) == 0, -1);

    memset(buf, 0, sizeof(buf));
    expect("fork pipe read", read(fds[0], buf, sizeof(msg) - 1) == (ssize_t)(sizeof(msg) - 1), -1);
    expect("fork pipe content", memcmp(buf, msg, sizeof(msg) - 1) == 0, -1);
    close(fds[0]);

    int status = -1;
    pid_t waited = waitpid(pid, &status, 0);
    expect("waitpid child", waited == pid && status == 23, status);
    expect("fork cow stack", cow_value == 0x11112222, cow_value);
    expect("fork sem_close", sem_close(sem) == 0, -1);

    return failed == local_failed;
}

static int test_memory_and_sleep(void)
{
    int local_failed = failed;

    void *ptr = malloc(128);
    expect("malloc", ptr != NULL, 0);
    if (ptr != NULL) {
        memset(ptr, 0xab, 128);
        free(ptr);
        ok("free");
    }

    polyos_sleep(1);
    ok("sleep");

    expect("getpid", getpid() >= 0, -1);
    expect("getppid", getppid() >= 0, -1);

    return failed == local_failed;
}

static int wait_for_dhcp_bound(struct network_info *info)
{
    for (int i = 0; i < 5000; i++) {
        if (network_info(info) == 0 && info->present && info->dhcp_state == 3) {
            return 1;
        }
        polyos_sleep(1);
    }

    return 0;
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
            polyos_sleep(1);
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
    test_devices();
    test_file_io();
    test_pipe();
    test_semaphore_basic();
    test_fork_pipe_semaphore();

    if (wants_network(argc, argv)) {
        test_network();
    } else {
        printf("[skip] network (run /bin/selftest.elf net)\n");
    }

    printf("selftest: passed=%d failed=%d\n", passed, failed);
    return failed == 0 ? 0 : 1;
}
