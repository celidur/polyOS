#include <errno.h>
#include <polyos.h>
extern long __polyos_syscall6(long, long, long, long, long, long, long);
void test_std_abi(void (*expect)(const char *, int, int)) {
  char random[32];
  expect("getrandom insecure hash seed",
         __polyos_syscall6(355, (long)random, sizeof(random), 4, 0, 0, 0) ==
             sizeof(random),
         1);
  expect("getrandom rejects invalid flags",
         __polyos_syscall6(355, (long)random, sizeof(random), 8, 0, 0, 0) ==
             -EINVAL,
         1);
  expect("getrandom validates user address",
         __polyos_syscall6(355, 1, 32, 4, 0, 0, 0) == -EFAULT, 1);
  long secure =
      __polyos_syscall6(355, (long)random, sizeof(random), 0, 0, 0, 0);
  expect("getrandom secure entropy or explicit unavailable",
         secure == sizeof(random) || secure == -EAGAIN, 1);
  int pair[2];
  if (socketpair(AF_UNIX, SOCK_STREAM, 0, pair) == 0) {
    int alias = dup(pair[0]), enabled = 1;
    expect("FIONBIO sets shared status flags",
           ioctl(alias, 0x5421, (unsigned long)&enabled) == 0 &&
               (fcntl(pair[0], F_GETFL, 0) & O_NONBLOCK),
           1);
    errno = 0;
    expect("FIONBIO empty read is nonblocking",
           read(pair[0], random, 1) == -1 && errno == EAGAIN, 1);
    close(alias);
    close(pair[0]);
    close(pair[1]);
  }
  int word = 0;
  struct timespec deadline;
  clock_gettime(CLOCK_MONOTONIC, &deadline);
  deadline.tv_nsec += 20000000;
  if (deadline.tv_nsec >= 1000000000) {
    deadline.tv_sec++;
    deadline.tv_nsec -= 1000000000;
  }
  expect("futex bitset absolute deadline",
         __polyos_syscall6(240, (long)&word, 9 | 128, 0, (long)&deadline, 0,
                           -1) == -ETIMEDOUT,
         1);
  expect("futex partial mask explicitly unsupported",
         __polyos_syscall6(240, (long)&word, 9 | 128, 0, 0, 0, 1) == -ENOTSUP,
         1);
}
