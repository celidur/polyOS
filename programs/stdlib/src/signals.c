#include "syscall_internal.h"
#include <polyos.h>
int sigprocmask(int how, const sigset_t *set, sigset_t *old) {
  return call6(126, how, (long)set, (long)old, 0, 0, 0);
}
int pthread_sigmask(int how, const sigset_t *set, sigset_t *old) {
  long r = __polyos_syscall6(126, how, (long)set, (long)old, 0, 0, 0);
  return r < 0 ? -r : 0;
}
int sigpending(sigset_t *set) { return call6(73, (long)set, 0, 0, 0, 0, 0); }
int sigaltstack(const stack_t *stack, stack_t *old) {
  return call6(186, (long)stack, (long)old, 0, 0, 0, 0);
}
int sigemptyset(sigset_t *s) {
  if (!s) {
    errno = EINVAL;
    return -1;
  }
  *s = 0;
  return 0;
}
int sigfillset(sigset_t *s) {
  if (!s) {
    errno = EINVAL;
    return -1;
  }
  *s = 0x7fffffff;
  return 0;
}
static int valid(sigset_t *s, int n) {
  if (!s || n < 1 || n > 31) {
    errno = EINVAL;
    return 0;
  }
  return 1;
}
int sigaddset(sigset_t *s, int n) {
  if (!valid(s, n))
    return -1;
  *s |= 1u << (n - 1);
  return 0;
}
int sigdelset(sigset_t *s, int n) {
  if (!valid(s, n))
    return -1;
  *s &= ~(1u << (n - 1));
  return 0;
}
int sigismember(const sigset_t *s, int n) {
  if (!valid((sigset_t *)s, n))
    return -1;
  return (*s >> (n - 1)) & 1;
}
int pause(void) { return call6(29, 0, 0, 0, 0, 0, 0); }
int raise(int signal) { return kill(getpid(), signal); }
