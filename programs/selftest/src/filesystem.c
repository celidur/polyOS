#include <errno.h>
#include <polyos.h>
#include <stdio.h>
#include <string.h>
static void truncation(const char *path,
                       void (*expect)(const char *, int, int)) {
  int fd = open(path, O_CREAT | O_TRUNC | O_RDWR, 0600);
  char buf[16];
  expect("truncate fixture", fd >= 0 && write(fd, "abcdef", 6) == 6, 1);
  expect("ftruncate shrink", ftruncate(fd, 2) == 0, 1);
  expect("truncate preserves offset", lseek(fd, 0, SEEK_CUR) == 6, 1);
  expect("read past truncated EOF", read(fd, buf, 1) == 0, 1);
  expect("ftruncate extend", ftruncate(fd, 8) == 0, 1);
  expect("extension preserves offset", lseek(fd, 0, SEEK_CUR) == 6, 1);
  lseek(fd, 0, SEEK_SET);
  int n = read(fd, buf, 8);
  expect("truncate zero fills",
         n == 8 && buf[0] == 'a' && buf[1] == 'b' && buf[2] == 0 && buf[7] == 0,
         1);
  lseek(fd, 12, SEEK_SET);
  expect("sparse offset write", write(fd, "Z", 1) == 1, 1);
  lseek(fd, 8, SEEK_SET);
  n = read(fd, buf, 5);
  expect("sparse gap zero fills",
         n == 5 && buf[0] == 0 && buf[3] == 0 && buf[4] == 'Z', 1);
  struct stat64 st;
  expect("fstat64 size",
         fstat64(fd, &st) == 0 && st.st_size == 13 && st.st_blksize == 512, 1);
  close(fd);
  fd = open(path, O_RDONLY, 0);
  errno = 0;
  expect("ftruncate rejects readonly", ftruncate(fd, 0) == -1 && errno == EBADF,
         1);
  errno = 0;
  expect("write rejects readonly", write(fd, "X", 1) == -1 && errno == EBADF,
         1);
  close(fd);
  unlink(path);
}
void test_filesystem(void (*expect)(const char *, int, int)) {
  truncation("/tmp/truncate-test", expect);
  truncation("/truncate-test", expect);
  mkdir("/tmp/at-test", 0700);
  int dir = open("/tmp/at-test", O_RDONLY | O_DIRECTORY, 0);
  int fd = openat(dir, "child", O_CREAT | O_EXCL | O_RDWR | O_CLOEXEC, 0600);
  expect("openat relative directory", dir >= 0 && fd >= 0, 1);
  expect("openat cloexec", fcntl(fd, F_GETFD, 0) == FD_CLOEXEC, 1);
  errno = 0;
  expect("openat exclusive",
         openat(dir, "child", O_CREAT | O_EXCL | O_RDWR, 0600) == -1 &&
             errno == EEXIST,
         1);
  errno = 0;
  expect("openat invalid directory fd",
         openat(127, "child", 0, 0) == -1 && errno == EBADF, 1);
  errno = 0;
  expect("openat file is not directory",
         openat(fd, "child", 0, 0) == -1 && errno == ENOTDIR, 1);
  int absolute = openat(127, "/tmp/at-test/child", O_RDONLY, 0);
  expect("openat absolute ignores dirfd", absolute >= 0, 1);
  close(absolute);
  struct file_stat st;
  struct stat64 wide;
  expect("fstatat relative",
         fstatat(dir, "child", &st, 0) == 0 && S_ISREG(st.mode), 1);
  expect("fstatat empty path",
         fstatat64(fd, "", &wide, AT_EMPTY_PATH) == 0 && S_ISREG(wide.st_mode),
         1);
  errno = 0;
  expect("fstatat rejects flags",
         fstatat64(fd, "", &wide, 0x400000) == -1 && errno == EINVAL, 1);
  expect("fchmod object",
         fchmod(fd, 0400) == 0 && fstat(fd, &st) == 0 &&
             (st.mode & 0777) == 0400,
         1);
  expect("access read", access("/tmp/at-test/child", R_OK) == 0, 1);
  errno = 0;
  expect("access denies write",
         access("/tmp/at-test/child", W_OK) == -1 && errno == EACCES, 1);
  errno = 0;
  expect("access rejects mode",
         access("/tmp/at-test/child", 8) == -1 && errno == EINVAL, 1);
  int second = openat(dir, "longer-name", O_CREAT | O_RDWR, 0600);
  close(second);
  // Directory handles snapshot entries when opened.
  close(dir);
  dir = open("/tmp/at-test", O_RDONLY, 0);
  char entries[256];
  errno = 0;
  expect("getdents64 tiny buffer",
         getdents64(dir, entries, 8) == -1 && errno == EINVAL, 1);
  int n = getdents64(dir, entries, sizeof(entries)), count = 0, valid = n > 0;
  for (int off = 0; off < n;) {
    struct dirent64 *d = (void *)(entries + off);
    if (d->d_reclen < 24 || d->d_reclen % 8 || off + d->d_reclen > n) {
      valid = 0;
      break;
    }
    count++;
    off += d->d_reclen;
  }
  expect("getdents64 variable records", valid && count == 2, 1);
  expect("getdents64 EOF", getdents64(dir, entries, sizeof(entries)) == 0, 1);
  unlink("/tmp/at-test/child");
  expect("fchmod survives unlink",
         fchmod(fd, 0600) == 0 && fstat(fd, &st) == 0 &&
             (st.mode & 0777) == 0600,
         1);
  close(fd);
  close(dir);
  unlink("/tmp/at-test/longer-name");
  rmdir("/tmp/at-test");
  int private_fd = open("/tmp/mode-zero", O_CREAT | O_RDWR, 0);
  expect("create mode zero",
         private_fd >= 0 && fstat(private_fd, &st) == 0 &&
             (st.mode & 0777) == 0,
         1);
  close(private_fd);
  errno = 0;
  expect("mode zero prevents reopen",
         open("/tmp/mode-zero", O_RDONLY, 0) == -1 && errno == EACCES, 1);
  unlink("/tmp/mode-zero");
  char exe[128];
  int len = readlink("/proc/self/exe", exe, sizeof(exe));
  expect("readlink executable",
         len == 17 && strncmp(exe, "/bin/selftest.elf", 17) == 0, 1);
  char tiny[4] = {'x', 'x', 'x', 'x'};
  expect("readlink truncates without NUL",
         readlink("/proc/self/exe", tiny, 3) == 3 && tiny[0] == '/' &&
             tiny[3] == 'x',
         1);
  expect("lstat64 link type",
         lstat64("/proc/self/exe", &wide) == 0 && S_ISLNK(wide.st_mode), 1);
  expect("stat64 follows link",
         stat64("/proc/self/exe", &wide) == 0 && S_ISREG(wide.st_mode) &&
             wide.st_size > 0,
         1);
  int exe_fd = open("/proc/self/exe", O_RDONLY, 0);
  expect("open follows executable link", exe_fd >= 0, 1);
  close(exe_fd);
  errno = 0;
  expect("readlink regular file",
         readlink("/bin/selftest.elf", exe, sizeof(exe)) == -1 &&
             errno == EINVAL,
         1);
  expect("stat64 ABI size", sizeof(struct stat64) == 96, 1);
}
