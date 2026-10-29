#include "syscall_internal.h"
#include <polyos.h>
#include <pthread.h>
static int futex(volatile u32 *p, int op, u32 value,
                 const struct timespec *timeout) {
  return __polyos_syscall6(240, (long)p, op | 128, value, (long)timeout, 0, 0);
}
int pthread_mutexattr_init(pthread_mutexattr_t *a) {
  if (!a)
    return EINVAL;
  a->kind = PTHREAD_MUTEX_DEFAULT;
  return 0;
}
int pthread_mutexattr_destroy(pthread_mutexattr_t *a) { return a ? 0 : EINVAL; }
int pthread_mutexattr_settype(pthread_mutexattr_t *a, int type) {
  if (!a || type < 0 || type > 2)
    return EINVAL;
  a->kind = type;
  return 0;
}
int pthread_mutex_init(pthread_mutex_t *m, const pthread_mutexattr_t *a) {
  if (!m || (a && (a->kind < 0 || a->kind > 2)))
    return EINVAL;
  *m = (pthread_mutex_t){0, 0, a ? a->kind : PTHREAD_MUTEX_DEFAULT};
  return 0;
}
int pthread_mutex_destroy(pthread_mutex_t *m) {
  return !m ? EINVAL : __atomic_load_n(&m->owner, __ATOMIC_ACQUIRE) ? EBUSY : 0;
}
int pthread_mutex_trylock(pthread_mutex_t *m) {
  if (!m)
    return EINVAL;
  u32 tid = pthread_self(), zero = 0;
  if (__atomic_compare_exchange_n(&m->owner, &zero, tid, 0, __ATOMIC_ACQUIRE,
                                  __ATOMIC_RELAXED)) {
    m->depth = 1;
    return 0;
  }
  if (zero == tid && m->kind == PTHREAD_MUTEX_RECURSIVE) {
    if (m->depth == (u32)-1)
      return EAGAIN;
    m->depth++;
    return 0;
  }
  return EBUSY;
}
int pthread_mutex_lock(pthread_mutex_t *m) {
  if (!m)
    return EINVAL;
  for (;;) {
    int r = pthread_mutex_trylock(m);
    if (!r)
      return 0;
    u32 owner = __atomic_load_n(&m->owner, __ATOMIC_RELAXED);
    if (owner == pthread_self() && m->kind == PTHREAD_MUTEX_ERRORCHECK)
      return EDEADLK;
    if (owner) {
      r = futex(&m->owner, 0, owner, 0);
      if (r < 0 && r != -EAGAIN && r != -EINTR)
        return -r;
    }
  }
}
int pthread_mutex_unlock(pthread_mutex_t *m) {
  if (!m)
    return EINVAL;
  if (__atomic_load_n(&m->owner, __ATOMIC_RELAXED) != pthread_self())
    return EPERM;
  if (--m->depth)
    return 0;
  __atomic_store_n(&m->owner, 0, __ATOMIC_RELEASE);
  futex(&m->owner, 1, 1, 0);
  return 0;
}
int pthread_condattr_init(pthread_condattr_t *a) {
  if (!a)
    return EINVAL;
  a->clock = CLOCK_REALTIME;
  return 0;
}
int pthread_condattr_destroy(pthread_condattr_t *a) { return a ? 0 : EINVAL; }
int pthread_cond_init(pthread_cond_t *c, const pthread_condattr_t *a) {
  if (!c)
    return EINVAL;
  if (a && a->clock != CLOCK_REALTIME)
    return ENOTSUP;
  *c = (pthread_cond_t){0, 0};
  return 0;
}
int pthread_cond_destroy(pthread_cond_t *c) {
  return !c                                               ? EINVAL
         : __atomic_load_n(&c->waiters, __ATOMIC_ACQUIRE) ? EBUSY
                                                          : 0;
}
static int cond_wait(pthread_cond_t *c, pthread_mutex_t *m,
                     const struct timespec *absolute) {
  if (!c || !m)
    return EINVAL;
  if (__atomic_load_n(&m->owner, __ATOMIC_RELAXED) != pthread_self() ||
      m->depth != 1)
    return EPERM;
  struct timespec relative, *timeout = 0;
  if (absolute) {
    if (absolute->tv_nsec < 0 || absolute->tv_nsec >= 1000000000)
      return EINVAL;
    struct timespec now;
    clock_gettime(CLOCK_REALTIME, &now);
    s64 seconds = (s64)absolute->tv_sec - now.tv_sec;
    int nanos = absolute->tv_nsec - now.tv_nsec;
    if (nanos < 0) {
      --seconds;
      nanos += 1000000000;
    }
    if (seconds < 0) {
      seconds = 0;
      nanos = 0;
    }
    if (seconds > 2147483647)
      seconds = 2147483647;
    relative = (struct timespec){(s32)seconds, nanos};
    timeout = &relative;
  }
  u32 sequence = __atomic_load_n(&c->sequence, __ATOMIC_ACQUIRE);
  __sync_fetch_and_add(&c->waiters, 1);
  pthread_mutex_unlock(m);
  int result = futex(&c->sequence, 0, sequence, timeout);
  __sync_fetch_and_sub(&c->waiters, 1);
  int r = pthread_mutex_lock(m);
  if (r)
    return r;
  return result == 0 || result == -EAGAIN || result == -EINTR ? 0 : -result;
}
int pthread_cond_wait(pthread_cond_t *c, pthread_mutex_t *m) {
  return cond_wait(c, m, 0);
}
int pthread_cond_timedwait(pthread_cond_t *c, pthread_mutex_t *m,
                           const struct timespec *t) {
  return t ? cond_wait(c, m, t) : EINVAL;
}
int pthread_cond_signal(pthread_cond_t *c) {
  if (!c)
    return EINVAL;
  __sync_fetch_and_add(&c->sequence, 1);
  futex(&c->sequence, 1, 1, 0);
  return 0;
}
int pthread_cond_broadcast(pthread_cond_t *c) {
  if (!c)
    return EINVAL;
  __sync_fetch_and_add(&c->sequence, 1);
  futex(&c->sequence, 1, 0x7fffffff, 0);
  return 0;
}
int pthread_once(pthread_once_t *once, void (*init)(void)) {
  if (!once || !init)
    return EINVAL;
  u32 zero = 0;
  if (__atomic_compare_exchange_n(once, &zero, 1, 0, __ATOMIC_ACQUIRE,
                                  __ATOMIC_RELAXED)) {
    init();
    __atomic_store_n(once, 2, __ATOMIC_RELEASE);
    futex(once, 1, 0x7fffffff, 0);
    return 0;
  }
  while (__atomic_load_n(once, __ATOMIC_ACQUIRE) != 2)
    futex(once, 0, 1, 0);
  return 0;
}
