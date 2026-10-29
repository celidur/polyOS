#ifndef POLYOS_PTHREAD_H
#define POLYOS_PTHREAD_H
#include "types.h"
#define PTHREAD_CREATE_JOINABLE 0
#define PTHREAD_CREATE_DETACHED 1
#define PTHREAD_STACK_MIN 16384

typedef u32 pthread_t;
typedef u32 pthread_key_t;
typedef struct {
    void *stackaddr;
    size_t stacksize;
    size_t guardsize;
    int detachstate;
} pthread_attr_t;

pthread_t pthread_self(void);
int pthread_equal(pthread_t a, pthread_t b);
int pthread_create(pthread_t *, const pthread_attr_t *, void *(*)(void *), void *);
int pthread_join(pthread_t, void **);
int pthread_detach(pthread_t);
void pthread_exit(void *) __attribute__((noreturn));
int pthread_key_create(pthread_key_t *, void (*)(void *));
int pthread_key_delete(pthread_key_t);
void *pthread_getspecific(pthread_key_t);
int pthread_setspecific(pthread_key_t, const void *);
int pthread_attr_init(pthread_attr_t *);
int pthread_attr_destroy(pthread_attr_t *);
int pthread_attr_setstacksize(pthread_attr_t *, size_t);
int pthread_attr_getstacksize(const pthread_attr_t *, size_t *);
int pthread_attr_setstack(pthread_attr_t *, void *, size_t);
int pthread_attr_getstack(const pthread_attr_t *, void **, size_t *);
int pthread_attr_setguardsize(pthread_attr_t *, size_t);
int pthread_attr_getguardsize(const pthread_attr_t *, size_t *);
int pthread_attr_setdetachstate(pthread_attr_t *, int);
int pthread_attr_getdetachstate(const pthread_attr_t *, int *);
int pthread_getattr_np(pthread_t, pthread_attr_t *);

#define PTHREAD_MUTEX_NORMAL 0
#define PTHREAD_MUTEX_RECURSIVE 1
#define PTHREAD_MUTEX_ERRORCHECK 2
#define PTHREAD_MUTEX_DEFAULT PTHREAD_MUTEX_ERRORCHECK
#define PTHREAD_PROCESS_PRIVATE 0
#define PTHREAD_PROCESS_SHARED 1
typedef struct { volatile u32 owner;u32 depth;int kind; } pthread_mutex_t;
typedef struct {int kind;} pthread_mutexattr_t;
typedef struct {volatile u32 sequence,waiters;} pthread_cond_t;
typedef struct {int clock;} pthread_condattr_t;
typedef volatile u32 pthread_once_t;
#define PTHREAD_MUTEX_INITIALIZER {0,0,PTHREAD_MUTEX_DEFAULT}
#define PTHREAD_COND_INITIALIZER {0,0}
#define PTHREAD_ONCE_INIT 0
int pthread_mutex_init(pthread_mutex_t *,const pthread_mutexattr_t *);
int pthread_mutex_destroy(pthread_mutex_t *);
int pthread_mutex_lock(pthread_mutex_t *);
int pthread_mutex_trylock(pthread_mutex_t *);
int pthread_mutex_unlock(pthread_mutex_t *);
int pthread_mutexattr_init(pthread_mutexattr_t *);
int pthread_mutexattr_destroy(pthread_mutexattr_t *);
int pthread_mutexattr_settype(pthread_mutexattr_t *,int);
int pthread_cond_init(pthread_cond_t *,const pthread_condattr_t *);
int pthread_cond_destroy(pthread_cond_t *);
int pthread_cond_wait(pthread_cond_t *,pthread_mutex_t *);
int pthread_cond_timedwait(pthread_cond_t *,pthread_mutex_t *,const struct timespec *);
int pthread_cond_signal(pthread_cond_t *);
int pthread_cond_broadcast(pthread_cond_t *);
int pthread_condattr_init(pthread_condattr_t *);
int pthread_condattr_destroy(pthread_condattr_t *);
int pthread_once(pthread_once_t *,void (*)(void));

#endif
