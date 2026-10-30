/* GNU/i386 C ABI adapter for the upstream Rust Unix standard library. */
#define sigaction __polyos_sigaction
#define sigprocmask __polyos_sigprocmask
#define pthread_sigmask __polyos_pthread_sigmask
#define sigemptyset __polyos_sigemptyset
#define sigfillset __polyos_sigfillset
#define sigaddset __polyos_sigaddset
#define sigdelset __polyos_sigdelset
#define sigismember __polyos_sigismember
#define tcgetattr __polyos_tcgetattr
#define tcsetattr __polyos_tcsetattr
#define opendir __polyos_opendir
#define readdir __polyos_readdir
#define closedir __polyos_closedir
#include <polyos.h>
#undef sigaction
#undef sigprocmask
#undef pthread_sigmask
#undef sigemptyset
#undef sigfillset
#undef sigaddset
#undef sigdelset
#undef sigismember
#include <stdio.h>
#undef opendir
#undef readdir
#undef closedir
#undef tcgetattr
#undef tcsetattr
#include "syscall_internal.h"
#include <memory.h>
#include <stdlib.h>
#include <string.h>
#define pthread_attr_init __polyos_pthread_attr_init
#define pthread_create __polyos_pthread_create
#include <pthread.h>
#undef pthread_attr_init
#undef pthread_create
int pthread_attr_init(pthread_attr_t *attr) {
  int r = __polyos_pthread_attr_init(attr);
  if (!r)
    attr->guardsize = 4096;
  return r;
}
int pthread_create(pthread_t *thread, const pthread_attr_t *attr,
                   void *(*start)(void *), void *arg) {
  pthread_attr_t defaults;
  if (!attr) {
    pthread_attr_init(&defaults);
    attr = &defaults;
  }
  return __polyos_pthread_create(thread, attr, start, arg);
}
extern char **environ;
extern unsigned long *__polyos_auxv;
extern int main(int, char **, char **);
void __polyos_std_start(int argc, char **argv, char **envp) {
  environ = envp;
  char **end = envp;
  while (*end)
    ++end;
  __polyos_auxv = (unsigned long *)(end + 1);
  _exit(main(argc, argv, envp));
}

int bcmp(const void *a, const void *b, size_t n) { return memcmp(a, b, n); }
void abort(void) {
  _exit(134);
  for (;;) {
  }
}
int open64(const char *path, int flags, int mode) {
  return open(path, flags, mode);
}
long long lseek64(int fd, long long offset, int whence) {
  if (offset < -2147483648LL || offset > 2147483647LL) {
    errno = 75;
    return -1;
  }
  return lseek(fd, offset, whence);
}
void *mmap64(void *addr, size_t size, int prot, int flags, int fd,
             long long offset) {
  if (offset < 0 || offset > 0xffffffffLL) {
    errno = 75;
    return MAP_FAILED;
  }
  return mmap(addr, size, prot, flags & ~(0x20000 | 0x4000), fd, offset);
}
struct gnu_sigaction {
  unsigned handler;
  unsigned mask[32];
  unsigned flags, restorer;
};
int sigemptyset(sigset_t *set) {
  if (!set) {
    errno = EINVAL;
    return -1;
  }
  memset(set, 0, 128);
  return 0;
}
int sigfillset(sigset_t *set) {
  if (sigemptyset(set))
    return -1;
  *set = 0x7fffffff;
  return 0;
}
int sigaddset(sigset_t *set, int sig) { return __polyos_sigaddset(set, sig); }
int sigdelset(sigset_t *set, int sig) { return __polyos_sigdelset(set, sig); }
int sigismember(const sigset_t *set, int sig) {
  return __polyos_sigismember(set, sig);
}
int sigprocmask(int how, const sigset_t *set, sigset_t *old) {
  sigset_t previous;
  int r = __polyos_sigprocmask(how, set, old ? &previous : 0);
  if (!r && old) {
    memset(old, 0, 128);
    *old = previous;
  }
  return r;
}
int pthread_sigmask(int how, const sigset_t *set, sigset_t *old) {
  int saved = errno, result = sigprocmask(how, set, old), error = errno;
  errno = saved;
  return result ? error : 0;
}
int sigaction(int sig, const struct gnu_sigaction *action,
              struct gnu_sigaction *old) {
  struct __polyos_sigaction input = {0}, output;
  if (action) {
    input.sa_handler = (sighandler_t)action->handler;
    input.sa_mask = action->mask[0];
    input.sa_flags = action->flags;
    input.sa_restorer = (void *)action->restorer;
  }
  int r = __polyos_sigaction(sig, action ? &input : 0, old ? &output : 0);
  if (!r && old) {
    memset(old, 0, sizeof(*old));
    old->handler = (unsigned)output.sa_handler;
    old->mask[0] = output.sa_mask;
    old->flags = output.sa_flags;
    old->restorer = (unsigned)output.sa_restorer;
  }
  return r;
}
int __xpg_strerror_r(int error, char *out, size_t len) {
  const char *message;
  switch (error) {
  case ENOENT:
    message = "No such file or directory";
    break;
  case EACCES:
    message = "Permission denied";
    break;
  case EINVAL:
    message = "Invalid argument";
    break;
  case EBADF:
    message = "Bad file descriptor";
    break;
  case ENOMEM:
    message = "Out of memory";
    break;
  case ENOTSUP:
  case ENOSYS:
    message = "Operation not supported";
    break;
  case EEXIST:
    message = "File exists";
    break;
  case ENOTDIR:
    message = "Not a directory";
    break;
  case EISDIR:
    message = "Is a directory";
    break;
  default:
    message = "Operating system error";
  }
  size_t n = strlen(message);
  if (!len)
    return ERANGE;
  if (n >= len) {
    memcpy(out, message, len - 1);
    out[len - 1] = 0;
    return ERANGE;
  }
  memcpy(out, message, n + 1);
  return 0;
}
char *realpath(const char *path, char *out) {
  struct stat64 st;
  if (stat64(path, &st))
    return 0;
  char buffer[4096], target[4096];
  int n = readlink(path, target, sizeof(target) - 1);
  if (n >= 0) {
    target[n] = 0;
    path = target;
  }
  if (*path == '/')
    buffer[0] = 0;
  else if (!getcwd(buffer, sizeof(buffer)))
    return 0;
  size_t used = strlen(buffer);
  for (const char *p = path; *p;) {
    while (*p == '/')
      ++p;
    const char *end = p;
    while (*end && *end != '/')
      ++end;
    size_t len = end - p;
    if (len == 2 && p[0] == '.' && p[1] == '.') {
      while (used && buffer[--used] != '/') {
      }
    } else if (len && !(len == 1 && *p == '.')) {
      if (used + len + 2 > sizeof(buffer)) {
        errno = ENAMETOOLONG;
        return 0;
      }
      buffer[used++] = '/';
      memcpy(buffer + used, p, len);
      used += len;
    }
    p = end;
  }
  if (!used)
    buffer[used++] = '/';
  buffer[used] = 0;
  if (!out) {
    out = malloc(used + 1);
    if (!out)
      return 0;
  }
  memcpy(out, buffer, used + 1);
  return out;
}

/* libc's userspace termios has 32 control characters and speed fields; the
 * kernel/legacy ABI has 19. Never expose uninitialized padding to Rust. */
int tcgetattr(int fd, struct termios *out) {
  struct termios value;
  if (__polyos_tcgetattr(fd, &value))
    return -1;
  memset(out, 0, 60);
  memcpy(out, &value, sizeof(value));
  return 0;
}
int tcsetattr(int fd, int action, const struct termios *value) {
  return __polyos_tcsetattr(fd, action, value);
}
struct gnu_dir {
  int fd, next, used;
  unsigned char records[4096];
};
void *fdopendir(int fd) {
  struct stat64 st;
  if (fstat64(fd, &st))
    return 0;
  if (!S_ISDIR(st.st_mode)) {
    errno = ENOTDIR;
    return 0;
  }
  struct gnu_dir *dir = calloc(1, sizeof(*dir));
  if (!dir)
    return 0;
  dir->fd = fd;
  return dir;
}
void *opendir(const char *path) {
  int fd = open(path, O_RDONLY | 0x10000 | 0x80000, 0);
  if (fd < 0)
    return 0;
  void *dir = fdopendir(fd);
  if (!dir) {
    int error = errno;
    close(fd);
    errno = error;
  }
  return dir;
}
int dirfd(void *ptr) { return ((struct gnu_dir *)ptr)->fd; }
void *readdir64(void *ptr) {
  struct gnu_dir *dir = ptr;
  if (dir->next >= dir->used) {
    dir->used = getdents64(dir->fd, dir->records, sizeof(dir->records));
    dir->next = 0;
    if (dir->used <= 0) {
      if (!dir->used)
        errno = 0;
      return 0;
    }
  }
  void *record = dir->records + dir->next;
  unsigned short size;
  memcpy(&size, (char *)record + 16, 2);
  if (size < 24 || dir->next + size > dir->used) {
    errno = EIO;
    return 0;
  }
  dir->next += size;
  return record;
}
int closedir(void *ptr) {
  if (!ptr) {
    errno = EINVAL;
    return -1;
  }
  int r = close(dirfd(ptr));
  free(ptr);
  return r;
}
int openat64(int fd, const char *path, int flags, int mode) {
  return openat(fd, path, flags, mode);
}
int ftruncate64(int fd, long long size) {
  if (size < 0 || size > 0x7fffffff) {
    errno = EINVAL;
    return -1;
  }
  return ftruncate(fd, size);
}
int clock_nanosleep(int clock, int flags, const struct timespec *request,
                    struct timespec *remaining) {
  if (flags & ~1)
    return EINVAL;
  struct timespec relative = *request;
  if (flags) {
    struct timespec now;
    if (clock_gettime(clock, &now))
      return errno;
    relative.tv_sec -= now.tv_sec;
    relative.tv_nsec -= now.tv_nsec;
    if (relative.tv_nsec < 0) {
      relative.tv_nsec += 1000000000;
      relative.tv_sec--;
    }
    if (relative.tv_sec < 0)
      return 0;
  }
  return nanosleep(&relative, remaining) ? errno : 0;
}
struct compat_iovec {
  void *base;
  size_t length;
};
ssize_t readv(int fd, const struct compat_iovec *vectors, int count) {
  if (count < 0 || count > 1024) {
    errno = EINVAL;
    return -1;
  }
  size_t total = 0;
  for (int i = 0; i < count; i++) {
    size_t room = 65536 - total;
    total += vectors[i].length < room ? vectors[i].length : room;
  }
  if (!total)
    return read(fd, 0, 0);
  char *buffer = malloc(total);
  if (!buffer)
    return -1;
  ssize_t n = read(fd, buffer, total);
  size_t done = 0;
  for (int i = 0; n > 0 && i < count && done < (size_t)n; i++) {
    size_t amount = vectors[i].length < (size_t)n - done ? vectors[i].length
                                                         : (size_t)n - done;
    memcpy(vectors[i].base, buffer + done, amount);
    done += amount;
  }
  free(buffer);
  return n;
}
const char *gnu_get_libc_version(void) {
  return "0.0";
} /* Disable glibc-specific optimizations. */
void *dlsym(void *handle, const char *name) {
  return 0;
} /* Static executables only. */
int getpwuid_r(unsigned uid, void *entry, char *buffer, size_t size,
               void **result) {
  *result = 0;
  return 0;
}

/* Explicit capability failures let std choose its read/write fallbacks. No
 * success is reported for filesystem metadata or IPC that PolyOS lacks. */
int statvfs(const char *p, void *out) {
  errno = ENOTSUP;
  return -1;
}
int linkat(int a, const char *b, int c, const char *d, int flags) {
  errno = ENOTSUP;
  return -1;
}
int symlink(const char *a, const char *b) {
  errno = ENOTSUP;
  return -1;
}
int rename(const char *a, const char *b) {
  errno = ENOTSUP;
  return -1;
}
int utimensat(int a, const char *b, const void *c, int d) {
  errno = ENOTSUP;
  return -1;
}
ssize_t recvmsg(int fd, void *msg, int flags) {
  errno = ENOTSUP;
  return -1;
}
ssize_t sendmsg(int fd, const void *msg, int flags) {
  errno = ENOTSUP;
  return -1;
}
ssize_t sendfile64(int a, int b, void *c, size_t d) {
  errno = ENOSYS;
  return -1;
}
ssize_t splice(int a, void *b, int c, void *d, size_t e, unsigned f) {
  errno = ENOSYS;
  return -1;
}
int unlinkat(int fd, const char *path, int flags) {
  return call6(301, fd, (long)path, flags, 0, 0, 0);
}
