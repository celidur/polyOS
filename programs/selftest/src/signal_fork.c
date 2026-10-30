#include <polyos.h>
static volatile int handler_child, forked;
static void fork_handler(int signal) {
  int child = fork();
  if (child == 0) handler_child = 1;
  else forked = child;
}
void test_signal_fork(void (*expect)(const char *, int, int)) {
  void *stack = mmap(0, 8192, 3, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  stack_t ss = {stack, 0, 8192};
  sigaltstack(&ss, 0);
  sigset_t mask = 1u << (SIGUSR2 - 1);
  sigprocmask(SIG_BLOCK, &mask, 0);
  kill(getpid(), SIGUSR2);
  int child = fork(), status = -1;
  if (child == 0) {
    stack_t current;
    sigset_t pending;
    sigaltstack(0, &current);
    sigpending(&pending);
    _exit(current.ss_sp == stack && current.ss_flags == 0 && current.ss_size == 8192 && !(pending & mask) ? 0 : 93);
  }
  expect("fork inherits alternate stack without pending signals",
         child > 0 && waitpid(child, &status, 0) == child && status == 0, 1);
  signal(SIGUSR2, SIG_IGN);
  sigprocmask(SIG_UNBLOCK, &mask, 0);
  struct sigaction action = {0};
  action.sa_handler = fork_handler;
  action.sa_flags = SA_ONSTACK;
  sigaction(SIGUSR1, &action, 0);
  kill(getpid(), SIGUSR1);
  if (handler_child) {
    stack_t current;
    sigset_t current_mask;
    sigaltstack(0, &current);
    sigprocmask(SIG_SETMASK, 0, &current_mask);
    _exit(current.ss_sp == stack && current.ss_flags == 0 &&
          !(current_mask & (1u << (SIGUSR1 - 1))) ? 0 : 94);
  }
  expect("fork child returns through inherited active signal frame",
         forked > 0 && waitpid(forked, &status, 0) == forked && status == 0, 1);
  signal(SIGUSR1, SIG_DFL);
  signal(SIGUSR2, SIG_DFL);
  ss.ss_flags = SS_DISABLE;
  sigaltstack(&ss, 0);
  munmap(stack, 8192);
}
