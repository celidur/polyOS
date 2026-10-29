#include "syscall_internal.h"
#include <memory.h>
#include <polyos.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>
int setuid(uid_t id) { return call6(23, id, 0, 0, 0, 0, 0); }
int setgid(gid_t id) { return call6(46, id, 0, 0, 0, 0, 0); }
int setgroups(size_t n, const gid_t *groups) {
  return call6(81, n, (long)groups, 0, 0, 0, 0);
}
int getgroups(int n, gid_t *groups) {
  return call6(80, n, (long)groups, 0, 0, 0, 0);
}
int chroot(const char *path) { return call6(61, (long)path, 0, 0, 0, 0, 0); }
int setpgid(pid_t p, pid_t group) { return call6(57, p, group, 0, 0, 0, 0); }
pid_t getpgid(pid_t p) { return call6(132, p, 0, 0, 0, 0, 0); }
pid_t getpgrp(void) { return call6(65, 0, 0, 0, 0, 0, 0); }
pid_t getsid(pid_t p) { return call6(147, p, 0, 0, 0, 0, 0); }
pid_t setsid(void) { return call6(66, 0, 0, 0, 0, 0, 0); }
int killpg(pid_t group, int signal) {
  if (group < 0) {
    errno = EINVAL;
    return -1;
  }
  return kill(-group, signal);
}
int prctl(int option, ...) {
  va_list ap;
  va_start(ap, option);
  long a = 0, b = 0, c = 0, d = 0;
  if (option == PR_SET_NAME || option == PR_GET_NAME)
    a = va_arg(ap, long);
  else if (option == PR_SET_NO_NEW_PRIVS || option == PR_GET_NO_NEW_PRIVS) {
    a = va_arg(ap, long);
    b = va_arg(ap, long);
    c = va_arg(ap, long);
    d = va_arg(ap, long);
  }
  va_end(ap);
  return call6(172, option, a, b, c, d, 0);
}
int execv(const char *path, char *const argv[]) {
  return execve(path, argv, environ);
}
int execvp(const char *file, char *const argv[]) {
  if (!file || !*file) {
    errno = ENOENT;
    return -1;
  }
  for (const char *p = file; *p; p++)
    if (*p == '/')
      return execv(file, argv);
  const char *path = getenv("PATH");
  if (!path)
    path = "/bin";
  int denied = 0;
  while (1) {
    const char *end = path;
    while (*end && *end != ':')
      end++;
    size_t n = end - path, m = strlen(file);
    if (n + m + 2 <= 256) {
      char candidate[256];
      size_t at = 0;
      if (n) {
        memcpy(candidate, path, n);
        candidate[n] = '/';
        at = n + 1;
      }
      memcpy(candidate + at, file, m + 1);
      execv(candidate, argv);
      if (errno == EACCES)
        denied = 1;
      else if (errno != ENOENT && errno != ENOTDIR)
        return -1;
    }
    if (!*end)
      break;
    path = end + 1;
  }
  errno = denied ? EACCES : ENOENT;
  return -1;
}
int waitid(idtype_t type, pid_t id, siginfo_t *info, int options) {
  return call6(284, type, id, (long)info, options, 0, 0);
}

int pthread_setname_np(unsigned thread, const char *name) {
  if (thread != (unsigned)gettid())
    return ENOTSUP;
  if (!name)
    return EINVAL;
  if (strlen(name) >= 16)
    return ERANGE;
  long r = __polyos_syscall6(172, 15, (long)name, 0, 0, 0, 0);
  return r < 0 ? -r : 0;
}
