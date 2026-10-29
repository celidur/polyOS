#include <errno.h>
#include <polyos.h>
#include <pthread.h>
static volatile int seen, on_alt, mask_in_handler;
static char alt[8192];
static void handler(int sig) {
  char local;
  seen++;
  on_alt = (uintptr_t)&local >= (uintptr_t)alt &&
           (uintptr_t)&local < (uintptr_t)alt + sizeof(alt);
  sigset_t mask;
  sigprocmask(SIG_SETMASK, 0, &mask);
  mask_in_handler = sigismember(&mask, sig);
}
static void *sender(void *arg) {
  struct timespec t = {0, 20000000};
  nanosleep(&t, 0);
  kill(getpid(), SIGUSR1);
  return 0;
}
static void *mask_worker(void *arg) {
  sigset_t m;
  sigprocmask(SIG_SETMASK, 0, &m);
  int inherited = sigismember(&m, SIGUSR2);
  sigdelset(&m, SIGUSR2);
  pthread_sigmask(SIG_SETMASK, &m, 0);
  return (void *)(uintptr_t)inherited;
}
void test_signal_runtime(void (*expect)(const char *, int, int)) {
  struct sigaction action = {handler, 0, 0, 0};
  expect("signal action", sigaction(SIGUSR1, &action, 0) == 0, 1);
  sigset_t set, old, pending;
  sigemptyset(&set);
  sigaddset(&set, SIGUSR1);
  expect("signal block", sigprocmask(SIG_BLOCK, &set, &old) == 0, 1);
  seen = 0;
  kill(getpid(), SIGUSR1);
  kill(getpid(), SIGUSR1);
  expect("blocked signals stay pending",
         seen == 0 && sigpending(&pending) == 0 &&
             sigismember(&pending, SIGUSR1),
         1);
  sigprocmask(SIG_UNBLOCK, &set, 0);
  expect("pending signals coalesce and deliver", seen == 1 && mask_in_handler,
         1);
  stack_t stack = {alt, 0, sizeof(alt)};
  expect("alternate signal stack", sigaltstack(&stack, 0) == 0, 1);
  action.sa_flags = SA_ONSTACK;
  sigaction(SIGUSR1, &action, 0);
  raise(SIGUSR1);
  expect("handler uses alternate stack", on_alt, 1);
  stack.ss_flags = SS_DISABLE;
  sigaltstack(&stack, 0);
  action.sa_flags = 0;
  sigaction(SIGUSR1, &action, 0);
  sigfillset(&set);
  sigprocmask(SIG_SETMASK, &set, 0);
  sigprocmask(SIG_SETMASK, 0, &pending);
  expect("KILL and STOP cannot be blocked",
         !sigismember(&pending, SIGKILL) && !sigismember(&pending, SIGSTOP), 1);
  sigprocmask(SIG_SETMASK, &old, 0);
  pthread_t t;
  pthread_create(&t, 0, sender, 0);
  errno = 0;
  expect("pause interrupted by handler", pause() == -1 && errno == EINTR, 1);
  pthread_join(t, 0);
  extern long __polyos_syscall6(long, long, long, long, long, long, long);
  u32 fake[16] = {0x53494731};
  expect("sigreturn rejects forged frame",
         __polyos_syscall6(119, (long)fake, 0, 0, 0, 0, 0) == -EINVAL, 1);

  sigemptyset(&set);
  sigaddset(&set, SIGUSR2);
  sigprocmask(SIG_BLOCK, &set, 0);
  void *result = 0;
  pthread_create(&t, 0, mask_worker, 0);
  pthread_join(t, &result);
  sigprocmask(SIG_SETMASK, 0, &pending);
  expect("thread masks inherit and stay independent",
         result == (void *)1 && sigismember(&pending, SIGUSR2), 1);
  sigprocmask(SIG_UNBLOCK, &set, 0);
  sigemptyset(&set);
  sigaddset(&set, SIGUSR1);
  expect("mask input output may alias",
         sigprocmask(SIG_BLOCK, &set, &set) == 0 && !sigismember(&set, SIGUSR1),
         1);
  sigprocmask(SIG_SETMASK, &old, 0);
  pthread_create(&t, 0, sender, 0);
  struct timespec request = {0, 100000000}, remaining = {0, 0};
  errno = 0;
  expect("nanosleep interrupt reports remainder",
         nanosleep(&request, &remaining) == -1 && errno == EINTR &&
             remaining.tv_nsec > 0 && remaining.tv_nsec < 100000000,
         1);
  pthread_join(t, 0);
  int fds[2];
  pipe2(fds, O_NONBLOCK);
  int child = fork();
  if (child == 0) {
    close(fds[0]);
    struct timespec delay = {0, 20000000};
    nanosleep(&delay, 0);
    write(fds[1], "X", 1);
    _exit(0);
  }
  close(fds[1]);
  kill(child, SIGSTOP);
  struct timespec delay = {0, 50000000};
  nanosleep(&delay, 0);
  char c;
  errno = 0;
  expect("STOP suspends the process",
         read(fds[0], &c, 1) == -1 && errno == EAGAIN, 1);
  kill(child, SIGCONT);
  struct pollfd polled = {fds[0], POLLIN, 0};
  expect("CONT resumes the process",
         poll(&polled, 1, 1000) == 1 && read(fds[0], &c, 1) == 1 && c == 'X',
         1);
  int status;
  waitpid(child, &status, 0);
  close(fds[0]);
  signal(SIGUSR1, SIG_DFL);
}
