#include <errno.h>
#include <polyos.h>
#include <stdio.h>
void test_virtual_memory(void (*expect)(const char *, int, int)) {
  unsigned char *p = mmap(0, 3 * 4096, PROT_READ | PROT_WRITE,
                          MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  expect("mmap anonymous", p != MAP_FAILED && !((uintptr_t)p & 4095), 1);
  if (p == MAP_FAILED)
    return;
  expect("mmap zero initialized", p[0] == 0 && p[8192] == 0, 1);
  p[0] = 42;
  p[4096] = 43;
  expect("mprotect readonly", mprotect(p, 4096, PROT_READ) == 0, 1);
  int child = fork(), status = 0;
  if (child == 0) {
    if (mprotect(p, 4096, PROT_READ | PROT_WRITE))
      _exit(98);
    p[0] = 99;
    _exit(p[0] == 99 ? 0 : 97);
  }
  expect("mmap fork COW after mprotect",
         child > 0 && waitpid(child, &status, 0) == child &&
             WIFEXITED(status) && WEXITSTATUS(status) == 0 && p[0] == 42,
         1);
  child = fork();
  if (child == 0) {
    p[0] = 99;
    _exit(0);
  }
  waitpid(child, &status, 0);
  expect("readonly page faults", status != 0, 1);
  expect("mprotect none", mprotect(p, 4096, PROT_NONE) == 0, 1);
  errno = 0;
  expect("PROT_NONE rejected by usercopy",
         write(1, p, 1) == -1 && errno == EFAULT, 1);
  expect("mprotect restores data",
         mprotect(p, 4096, PROT_READ | PROT_WRITE) == 0 && p[0] == 42, 1);
  p[0] = 44;
  expect("munmap middle page", munmap(p + 4096, 4096) == 0, 1);
  errno = 0;
  expect("mprotect hole rejected",
         mprotect(p, 3 * 4096, PROT_NONE) == -1 && errno == ENOMEM &&
             p[0] == 44,
         1);
  unsigned char *q = mmap(p + 4096, 4096, PROT_READ | PROT_WRITE,
                          MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED, -1, 0);
  expect("MAP_FIXED fills hole", q == p + 4096 && q[0] == 0, 1);
  q[0] = 9;
  q = mmap(q, 4096, PROT_READ | PROT_WRITE,
           MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED, -1, 0);
  expect("MAP_FIXED replaces mapping", q == p + 4096 && q[0] == 0, 1);
  expect("munmap range", munmap(p, 3 * 4096) == 0, 1);
  errno = 0;
  expect("munmap bad alignment", munmap(p + 1, 4096) == -1 && errno == EINVAL,
         1);
  errno = 0;
  expect("mmap rejects kernel address",
         mmap((void *)0x1000000, 4096, 3,
              MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED, -1, 0) == MAP_FAILED &&
             errno == EINVAL,
         1);
  int fd = open("/tmp/mmap-file", O_CREAT | O_RDWR | O_TRUNC, 0600);
  write(fd, "hello", 5);
  lseek(fd, 2, SEEK_SET);
  p = mmap(0, 4096, 3, MAP_PRIVATE, fd, 0);
  expect("file private mapping",
         p != MAP_FAILED && p[0] == 'h' && p[4] == 'o' && p[5] == 0, 1);
  expect("mmap preserves fd position", lseek(fd, 0, SEEK_CUR) == 2, 1);
  if (p != MAP_FAILED) {
    p[0] = 'X';
    munmap(p, 4096);
  }
  lseek(fd, 0, SEEK_SET);
  char c;
  expect("private mapping leaves file unchanged",
         read(fd, &c, 1) == 1 && c == 'h', 1);
  close(fd);
  unlink("/tmp/mmap-file");
}
