#include <errno.h>
#include <memory.h>
#include <polyos.h>
#include <stdlib.h>
#include <string.h>
struct action {
  int kind, fd, other, flags, mode;
  char *path;
};
int posix_spawn_file_actions_init(posix_spawn_file_actions_t *a) {
  if (!a)
    return EINVAL;
  *a = (posix_spawn_file_actions_t){0, 0};
  return 0;
}
int posix_spawn_file_actions_destroy(posix_spawn_file_actions_t *a) {
  if (!a)
    return EINVAL;
  struct action *v = a->actions;
  for (unsigned i = 0; i < a->count; i++)
    free(v[i].path);
  free(v);
  *a = (posix_spawn_file_actions_t){0, 0};
  return 0;
}
static int add(posix_spawn_file_actions_t *a, struct action item,
               const char *path) {
  if (!a || item.fd < 0 || item.fd >= 128 || item.other < 0 ||
      item.other >= 128)
    return EBADF;
  if (a->count >= 32)
    return E2BIG;
  if (path) {
    size_t n = strlen(path) + 1;
    item.path = malloc(n);
    if (!item.path)
      return ENOMEM;
    memcpy(item.path, path, n);
  }
  void *v = realloc(a->actions, (a->count + 1) * sizeof(item));
  if (!v) {
    free(item.path);
    return ENOMEM;
  }
  a->actions = v;
  ((struct action *)v)[a->count++] = item;
  return 0;
}
int posix_spawn_file_actions_addopen(posix_spawn_file_actions_t *a, int fd,
                                     const char *path, int flags, int mode) {
  if (!path)
    return EINVAL;
  return add(a, (struct action){1, fd, 0, flags, mode, 0}, path);
}
int posix_spawn_file_actions_addclose(posix_spawn_file_actions_t *a, int fd) {
  return add(a, (struct action){2, fd, 0, 0, 0, 0}, 0);
}
int posix_spawn_file_actions_adddup2(posix_spawn_file_actions_t *a, int fd,
                                     int target) {
  return add(a, (struct action){3, fd, target, 0, 0, 0}, 0);
}
int posix_spawn_file_actions_addchdir_np(posix_spawn_file_actions_t *a,
                                         const char *path) {
  if (!path)
    return EINVAL;
  return add(a, (struct action){4, 0, 0, 0, 0, 0}, path);
}
int posix_spawnattr_init(posix_spawnattr_t *a) {
  if (!a)
    return EINVAL;
  *a = (posix_spawnattr_t){0, 0, 0, 0};
  return 0;
}
int posix_spawnattr_destroy(posix_spawnattr_t *a) { return a ? 0 : EINVAL; }
int posix_spawnattr_setflags(posix_spawnattr_t *a, short f) {
  if (!a || f & ~(1 | 2 | 4 | 8 | 128))
    return EINVAL;
  a->flags = f;
  return 0;
}
int posix_spawnattr_getflags(const posix_spawnattr_t *a, short *f) {
  if (!a || !f)
    return EINVAL;
  *f = a->flags;
  return 0;
}
int posix_spawnattr_setpgroup(posix_spawnattr_t *a, pid_t p) {
  if (!a || p < 0)
    return EINVAL;
  a->group = p;
  return 0;
}
int posix_spawnattr_getpgroup(const posix_spawnattr_t *a, pid_t *p) {
  if (!a || !p)
    return EINVAL;
  *p = a->group;
  return 0;
}
int posix_spawnattr_setsigmask(posix_spawnattr_t *a, const sigset_t *s) {
  if (!a || !s)
    return EINVAL;
  a->mask = *s;
  return 0;
}
int posix_spawnattr_getsigmask(const posix_spawnattr_t *a, sigset_t *s) {
  if (!a || !s)
    return EINVAL;
  *s = a->mask;
  return 0;
}
int posix_spawnattr_setsigdefault(posix_spawnattr_t *a, const sigset_t *s) {
  if (!a || !s)
    return EINVAL;
  a->defaults = *s;
  return 0;
}
int posix_spawnattr_getsigdefault(const posix_spawnattr_t *a, sigset_t *s) {
  if (!a || !s)
    return EINVAL;
  *s = a->defaults;
  return 0;
}
static int referred(const posix_spawn_file_actions_t *a, int fd) {
  if (!a)
    return 0;
  struct action *v = a->actions;
  for (unsigned i = 0; i < a->count; i++)
    if (v[i].fd == fd || (v[i].kind == 3 && v[i].other == fd))
      return 1;
  return 0;
}
static void child_fail(int fd) {
  int error = errno;
  while (write(fd, &error, sizeof(error)) < 0 && errno == EINTR) {
  }
  _exit(127);
}
static int spawn(pid_t *out, const char *file,
                 const posix_spawn_file_actions_t *actions,
                 const posix_spawnattr_t *attr, char *const argv[],
                 char *const envp[], int search) {
  if (!out || !file || !argv)
    return EINVAL;
  if (attr && attr->flags & ~(1 | 2 | 4 | 8 | 128))
    return EINVAL;
  const char *search_path = getenv("PATH");
  int pipes[2];
  if (pipe2(pipes, O_CLOEXEC) < 0)
    return errno;
  int errorfd = -1;
  for (int minimum = 127; minimum >= 3; minimum--) {
    if (referred(actions, minimum))
      continue;
    int fd = fcntl(pipes[1], F_DUPFD_CLOEXEC, minimum);
    if (fd < 0)
      continue;
    if (referred(actions, fd)) {
      close(fd);
      continue;
    }
    errorfd = fd;
    break;
  }
  if (errorfd < 0) {
    close(pipes[0]);
    close(pipes[1]);
    return EMFILE;
  }
  close(pipes[1]);
  pid_t child = fork();
  if (child < 0) {
    int error = errno;
    close(pipes[0]);
    close(errorfd);
    return error;
  }
  if (child == 0) {
    close(pipes[0]);
    if (attr) {
      if (attr->flags & POSIX_SPAWN_RESETIDS) {
        if (setgid(getgid()) || setuid(getuid()))
          child_fail(errorfd);
      }
      if ((attr->flags & POSIX_SPAWN_SETSID) && setsid() < 0)
        child_fail(errorfd);
      if ((attr->flags & POSIX_SPAWN_SETPGROUP) && setpgid(0, attr->group) < 0)
        child_fail(errorfd);
      if (attr->flags & POSIX_SPAWN_SETSIGDEF) {
        for (int s = 1; s <= 31; s++)
          if ((attr->defaults >> (s - 1)) & 1) {
            if (signal(s, SIG_DFL) == SIG_ERR)
              child_fail(errorfd);
          }
      }
      if ((attr->flags & POSIX_SPAWN_SETSIGMASK) &&
          sigprocmask(SIG_SETMASK, &attr->mask, 0) < 0)
        child_fail(errorfd);
    }
    if (actions) {
      struct action *v = actions->actions;
      for (unsigned i = 0; i < actions->count; i++) {
        struct action *a = &v[i];
        if (a->kind == 1) {
          int fd = open(a->path, a->flags, a->mode);
          if (fd < 0)
            child_fail(errorfd);
          if (fd != a->fd) {
            if (dup2(fd, a->fd) < 0)
              child_fail(errorfd);
            close(fd);
          } else if (fcntl(fd, F_SETFD, 0) < 0)
            child_fail(errorfd);
        } else if (a->kind == 2) {
          if (close(a->fd) < 0)
            child_fail(errorfd);
        } else if (a->kind == 3) {
          if (dup2(a->fd, a->other) < 0 || fcntl(a->other, F_SETFD, 0) < 0)
            child_fail(errorfd);
        } else if (a->kind == 4) {
          if (chdir(a->path) < 0)
            child_fail(errorfd);
        }
      }
    }
    if (search) {
      extern int __polyos_exec_search(const char *, char *const[],
                                      char *const[], const char *);
      __polyos_exec_search(file, argv, envp, search_path);
    } else
      execve(file, argv, envp);
    child_fail(errorfd);
  }
  close(errorfd);
  int error = 0;
  ssize_t n;
  do {
    n = read(pipes[0], &error, sizeof(error));
  } while (n < 0 && errno == EINTR);
  int saved = errno;
  close(pipes[0]);
  if (n == 0) {
    *out = child;
    return 0;
  }
  if (n > 0) {
    while (waitpid(child, 0, 0) < 0 && errno == EINTR) {
    }
    return error ? error : EIO;
  }
  return saved;
}
int posix_spawn(pid_t *p, const char *f, const posix_spawn_file_actions_t *a,
                const posix_spawnattr_t *t, char *const v[], char *const e[]) {
  return spawn(p, f, a, t, v, e, 0);
}
int posix_spawnp(pid_t *p, const char *f, const posix_spawn_file_actions_t *a,
                 const posix_spawnattr_t *t, char *const v[], char *const e[]) {
  return spawn(p, f, a, t, v, e, 1);
}
