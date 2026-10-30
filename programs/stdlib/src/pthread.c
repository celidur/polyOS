#include "pthread.h"
#include "memory.h"
#include "polyos.h"
#include "stdlib.h"
#include "string.h"
#include "syscall_internal.h"

#define MAX_THREADS 64
#define MAX_KEYS 64
#define DEFAULT_STACK (64 * 1024)
#define CLONE_FLAGS                                                            \
  (0x100 | 0x200 | 0x400 | 0x800 | 0x10000 | 0x100000 | 0x200000)

/* The kernel clears live_tid only after the thread stops using its stack. */
struct thread_control {
  pthread_t tid;
  volatile u32 live_tid;
  int error;
  int detached;
  int joining;
  int owns_stack;
  void *stack_mapping;
  size_t stack_mapping_size;
  pthread_attr_t attr;
  void *(*start)(void *);
  void *arg;
  void *result;
  void *values[MAX_KEYS];
};
static struct thread_control main_thread;
static struct thread_control *threads[MAX_THREADS];
static void (*destructors[MAX_KEYS])(void *);
static unsigned char keys[MAX_KEYS];
static volatile int registry_lock;
static int initialized;
extern long __polyos_clone(long, void *, void *, void *, void *,
                           void (*)(void *), void *);

pid_t gettid(void) { return __polyos_syscall6(224, 0, 0, 0, 0, 0, 0); }
int sched_yield(void) { return call6(158, 0, 0, 0, 0, 0, 0); }
static void lock(void) {
  while (__sync_lock_test_and_set(&registry_lock, 1))
    sched_yield();
}
static void unlock(void) { __sync_lock_release(&registry_lock); }
static void init_locked(void) {
  if (!initialized) {
    main_thread.tid = gettid();
    main_thread.attr.stacksize = DEFAULT_STACK;
    threads[0] = &main_thread;
    initialized = 1;
  }
}
static struct thread_control *find_locked(pthread_t tid) {
  init_locked();
  for (unsigned i = 0; i < MAX_THREADS; ++i)
    if (threads[i] && threads[i]->tid == tid)
      return threads[i];
  return NULL;
}
struct errno_slot {
  volatile u32 owner;
  int value;
};
static struct errno_slot errno_slots[MAX_THREADS];
int *__pthread_errno_location(void) {
  u32 key = gettid() + 1;
  for (unsigned i = 0; i < MAX_THREADS; ++i)
    if (__atomic_load_n(&errno_slots[i].owner, __ATOMIC_ACQUIRE) == key)
      return &errno_slots[i].value;
  for (unsigned i = 0; i < MAX_THREADS; ++i) {
    u32 zero = 0;
    if (__atomic_compare_exchange_n(&errno_slots[i].owner, &zero, key, 0,
                                    __ATOMIC_ACQ_REL, __ATOMIC_RELAXED))
      return &errno_slots[i].value;
  }
  static int fallback;
  return &fallback;
}
static void release_errno(pthread_t tid) {
  for (unsigned i = 0; i < MAX_THREADS; ++i)
    if (__atomic_load_n(&errno_slots[i].owner, __ATOMIC_ACQUIRE) == tid + 1) {
      errno_slots[i].value = 0;
      __atomic_store_n(&errno_slots[i].owner, 0, __ATOMIC_RELEASE);
      return;
    }
}
static void remove_locked(struct thread_control *t) {
  for (unsigned i = 1; i < MAX_THREADS; ++i)
    if (threads[i] == t)
      threads[i] = NULL;
}
static void destroy(struct thread_control *t) {
  if (t->tid)
    release_errno(t->tid);
  if (t->stack_mapping)
    munmap(t->stack_mapping, t->stack_mapping_size);
  else if (t->owns_stack)
    free(t->attr.stackaddr);
  free(t);
}
static void reap_detached(void) {
  for (;;) {
    struct thread_control *dead = NULL;
    lock();
    for (unsigned i = 1; i < MAX_THREADS; ++i) {
      if (threads[i] && threads[i]->detached && !threads[i]->live_tid) {
        dead = threads[i];
        threads[i] = NULL;
        break;
      }
    }
    unlock();
    if (!dead)
      return;
    destroy(dead);
  }
}
pthread_t pthread_self(void) { return gettid(); }
int pthread_equal(pthread_t a, pthread_t b) { return a == b; }
static void start_thread(void *arg) {
  struct thread_control *t = arg;
  /* Wait for the creator to publish the registry entry and immutable tid. */
  lock();
  unlock();
  pthread_exit(t->start(t->arg));
}
int pthread_create(pthread_t *thread, const pthread_attr_t *attr,
                   void *(*start)(void *), void *arg) {
  if (!thread || !start)
    return EINVAL;
  pthread_attr_t selected;
  pthread_attr_init(&selected);
  if (attr)
    selected = *attr;
  if (selected.stacksize < PTHREAD_STACK_MIN ||
      selected.stacksize > 16 * 1024 * 1024 ||
      (selected.detachstate != 0 && selected.detachstate != 1))
    return EINVAL;
  if (selected.guardsize > 1024 * 1024 ||
      (selected.stackaddr && selected.guardsize))
    return EINVAL;
  reap_detached();
  struct thread_control *t = malloc(sizeof(*t));
  if (!t)
    return ENOMEM;
  memset(t, 0, sizeof(*t));
  t->attr = selected;
  if (!t->attr.stackaddr) {
    if (selected.guardsize) {
      size_t guard = (selected.guardsize + 4095) & ~4095u;
      size_t usable = (selected.stacksize + 4095) & ~4095u;
      void *mapping = mmap(0, guard + usable, PROT_READ | PROT_WRITE,
                           MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
      if (mapping == MAP_FAILED) {
        free(t);
        return ENOMEM;
      }
      if (mprotect(mapping, guard, PROT_NONE)) {
        munmap(mapping, guard + usable);
        free(t);
        return ENOMEM;
      }
      t->stack_mapping = mapping;
      t->stack_mapping_size = guard + usable;
      t->attr.stackaddr = (char *)mapping + guard;
      t->attr.stacksize = usable;
      t->attr.guardsize = guard;
    } else {
      t->attr.stackaddr = malloc(selected.stacksize);
    }
    if (!t->attr.stackaddr) {
      free(t);
      return ENOMEM;
    }
    t->owns_stack = 1;
  }
  t->start = start;
  t->arg = arg;
  t->detached = selected.detachstate;
  lock();
  init_locked();
  unsigned slot;
  for (slot = 1; slot < MAX_THREADS && threads[slot]; ++slot) {
  }
  if (slot == MAX_THREADS) {
    unlock();
    destroy(t);
    return EAGAIN;
  }
  threads[slot] = t;
  long tid = __polyos_clone(
      CLONE_FLAGS, (char *)t->attr.stackaddr + t->attr.stacksize,
      (void *)&t->live_tid, NULL, (void *)&t->live_tid, start_thread, t);
  if (tid < 0) {
    threads[slot] = NULL;
    unlock();
    destroy(t);
    return -tid;
  }
  t->tid = tid;
  *thread = tid;
  unlock();
  return 0;
}
int pthread_join(pthread_t thread, void **result) {
  if (thread == pthread_self())
    return EDEADLK;
  lock();
  struct thread_control *t = find_locked(thread);
  if (!t) {
    unlock();
    return ESRCH;
  }
  if (t == &main_thread || t->detached || t->joining) {
    unlock();
    return EINVAL;
  }
  t->joining = 1;
  unlock();
  for (;;) {
    u32 live = __atomic_load_n(&t->live_tid, __ATOMIC_ACQUIRE);
    if (!live)
      break;
    long r = __polyos_syscall6(240, (long)&t->live_tid, 0, live, 0, 0, 0);
    if (r < 0 && r != -EAGAIN && r != -EINTR) {
      lock();
      t->joining = 0;
      unlock();
      return -r;
    }
  }
  if (result)
    *result = t->result;
  lock();
  remove_locked(t);
  unlock();
  destroy(t);
  return 0;
}
int pthread_detach(pthread_t thread) {
  lock();
  struct thread_control *t = find_locked(thread);
  if (!t) {
    unlock();
    return ESRCH;
  }
  if (t == &main_thread || t->detached || t->joining) {
    unlock();
    return EINVAL;
  }
  t->detached = 1;
  unlock();
  reap_detached();
  return 0;
}
void pthread_exit(void *result) {
  lock();
  struct thread_control *t = find_locked(pthread_self());
  unlock();
  if (t) {
    for (int pass = 0; pass < 4; ++pass) {
      for (unsigned k = 0; k < MAX_KEYS; ++k) {
        lock();
        void *value = t->values[k];
        void (*dtor)(void *) = keys[k] ? destructors[k] : NULL;
        if (dtor && value)
          t->values[k] = NULL;
        unlock();
        if (dtor && value)
          dtor(value);
      }
    }
    t->result = result;
    __sync_synchronize();
  }
  __polyos_syscall6(1, 0, 0, 0, 0, 0, 0);
  for (;;) {
  }
}
int pthread_key_create(pthread_key_t *key, void (*dtor)(void *)) {
  if (!key)
    return EINVAL;
  lock();
  for (unsigned k = 0; k < MAX_KEYS; ++k)
    if (!keys[k]) {
      keys[k] = 1;
      destructors[k] = dtor;
      *key = k;
      unlock();
      return 0;
    }
  unlock();
  return EAGAIN;
}
int pthread_key_delete(pthread_key_t key) {
  lock();
  if (key >= MAX_KEYS || !keys[key]) {
    unlock();
    return EINVAL;
  }
  keys[key] = 0;
  destructors[key] = NULL;
  for (unsigned i = 0; i < MAX_THREADS; ++i)
    if (threads[i])
      threads[i]->values[key] = NULL;
  unlock();
  return 0;
}
void *pthread_getspecific(pthread_key_t key) {
  lock();
  struct thread_control *t = find_locked(pthread_self());
  void *value = t && key < MAX_KEYS && keys[key] ? t->values[key] : NULL;
  unlock();
  return value;
}
int pthread_setspecific(pthread_key_t key, const void *value) {
  lock();
  struct thread_control *t = find_locked(pthread_self());
  if (!t || key >= MAX_KEYS || !keys[key]) {
    unlock();
    return EINVAL;
  }
  t->values[key] = (void *)value;
  unlock();
  return 0;
}
int pthread_attr_init(pthread_attr_t *a) {
  if (!a)
    return EINVAL;
  *a = (pthread_attr_t){NULL, DEFAULT_STACK, 0, PTHREAD_CREATE_JOINABLE};
  return 0;
}
int pthread_attr_destroy(pthread_attr_t *a) { return a ? 0 : EINVAL; }
int pthread_attr_setstacksize(pthread_attr_t *a, size_t n) {
  if (!a || n < PTHREAD_STACK_MIN || n > 16 * 1024 * 1024)
    return EINVAL;
  a->stacksize = n;
  return 0;
}
int pthread_attr_getstacksize(const pthread_attr_t *a, size_t *n) {
  if (!a || !n)
    return EINVAL;
  *n = a->stacksize;
  return 0;
}
int pthread_attr_setstack(pthread_attr_t *a, void *p, size_t n) {
  if (!a || !p || n < PTHREAD_STACK_MIN || (uintptr_t)p + n < (uintptr_t)p)
    return EINVAL;
  a->stackaddr = p;
  a->stacksize = n;
  return 0;
}
int pthread_attr_getstack(const pthread_attr_t *a, void **p, size_t *n) {
  if (!a || !p || !n)
    return EINVAL;
  *p = a->stackaddr;
  *n = a->stacksize;
  return 0;
}
int pthread_attr_setguardsize(pthread_attr_t *a, size_t n) {
  if (!a)
    return EINVAL;
  if (n > 1024 * 1024)
    return EINVAL;
  a->guardsize = n;
  return 0;
}
int pthread_attr_getguardsize(const pthread_attr_t *a, size_t *n) {
  if (!a || !n)
    return EINVAL;
  *n = a->guardsize;
  return 0;
}
int pthread_attr_setdetachstate(pthread_attr_t *a, int state) {
  if (!a || (state != 0 && state != 1))
    return EINVAL;
  a->detachstate = state;
  return 0;
}
int pthread_attr_getdetachstate(const pthread_attr_t *a, int *state) {
  if (!a || !state)
    return EINVAL;
  *state = a->detachstate;
  return 0;
}
int pthread_getattr_np(pthread_t thread, pthread_attr_t *a) {
  if (!a)
    return EINVAL;
  lock();
  struct thread_control *t = find_locked(thread);
  if (t)
    *a = t->attr;
  unlock();
  return t ? 0 : ESRCH;
}
static struct thread_control *fork_caller;
static int fork_errno;
void __pthread_before_fork(void) {
  fork_errno = *__pthread_errno_location();
  lock();
  fork_caller = find_locked(pthread_self());
}
void __pthread_parent_after_fork(void) { unlock(); }
void __pthread_after_fork(void) {
  extern u32 __pthread_once_generation;
  __pthread_once_generation += 4;
  registry_lock = 0;
  memset(errno_slots, 0, sizeof(errno_slots));
  *__pthread_errno_location() = fork_errno;
  memset(threads, 0, sizeof(threads));
  if (fork_caller)
    main_thread = *fork_caller;
  main_thread.tid = gettid();
  main_thread.live_tid = 0;
  main_thread.owns_stack = 0;
  main_thread.stack_mapping = 0;
  main_thread.stack_mapping_size = 0;
  main_thread.joining = 0;
  main_thread.detached = 0;
  threads[0] = &main_thread;
  initialized = 1;
}
