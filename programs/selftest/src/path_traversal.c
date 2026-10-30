#include <errno.h>
#include <polyos.h>
#include <stdio.h>
void test_path_traversal(void (*expect)(const char *, int, int)) {
  mkdir("/tmp/walk-private", 0700);
  int fd = open("/tmp/walk-public", O_CREAT | O_RDWR, 0644);
  close(fd);
  int base = open("/tmp", O_RDONLY | O_DIRECTORY, 0);
  int child = fork(), status = -1;
  if (!child) {
    setuid(1000);
    errno = 0;
    fd = open("/tmp/walk-private/../walk-public", O_RDONLY, 0);
    int ok = fd == -1 && errno == EACCES;
    errno = 0;
    fd = openat(base, "walk-private/../walk-public", O_RDONLY, 0);
    ok &= fd == -1 && errno == EACCES;
    errno = 0;
    ok &= chmod("/tmp/walk-private/../walk-public", 0600) == -1 && errno == EACCES;
    _exit(ok ? 0 : 95);
  }
  expect("dotdot checks permissions before discarding components",
         child > 0 && waitpid(child, &status, 0) == child && status == 0, 1);
  errno = 0;
  fd = open("/tmp/walk-missing/../walk-public", O_RDONLY, 0);
  expect("dotdot checks missing component", fd == -1 && errno == ENOENT, 1);
  if (fd >= 0) close(fd);
  errno = 0;
  fd = open("/tmp/walk-public/../walk-public", O_RDONLY, 0);
  expect("dotdot rejects regular intermediate component", fd == -1 && errno == ENOTDIR, 1);
  if (fd >= 0) close(fd);
  errno = 0;
  fd = open("/tmp/walk-public/", O_RDONLY, 0);
  expect("trailing slash requires a directory", fd == -1 && errno == ENOTDIR, 1);
  if (fd >= 0) close(fd);
  fd = open("/proc/self/../self/status", O_RDONLY, 0);
  char byte;
  expect("procfs traversal runs outside task-manager lock", fd >= 0 && read(fd, &byte, 1) == 1, 1);
  if (fd >= 0) close(fd);
  child = fork();
  if (!child) {
    char *args[] = {"selftest.elf", "--thread-exec-child", 0};
    execve("/bin/walk-missing/../selftest.elf", args, 0);
    _exit(errno == ENOENT ? 0 : 96);
  }
  expect("exec checks traversal before dotdot normalization",
         child > 0 && waitpid(child, &status, 0) == child && status == 0, 1);
  close(base);
  unlink("/tmp/walk-public");
  rmdir("/tmp/walk-private");
}
