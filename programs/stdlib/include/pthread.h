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
#endif
