#include "errno.h"
#include "memory.h"
#include "polyos.h"
#include "pthread.h"
#include "stdlib.h"

static pthread_key_t worker_key;
static volatile int total;
static volatile int dtors;
static volatile int gate;
static volatile int detached_done;

static void destroy_value(void *value) {
    if (value)
        __sync_fetch_and_add(&dtors, 1);
}
static void *worker(void *arg) {
    uintptr_t id = (uintptr_t)arg;
    if (pthread_setspecific(worker_key, arg))
        return (void *)1;
    for (int i = 0; i < 32; ++i) {
        unsigned char *p = malloc(128);
        if (!p)
            return (void *)2;
        memset(p, (int)id, 128);
        errno = 0;
        if (close(-1) != -1 || errno != EBADF)
            return (void *)3;
        sched_yield();
        if (errno != EBADF || pthread_getspecific(worker_key) != arg || p[127] != (unsigned char)id)
            return (void *)4;
        free(p);
        __sync_fetch_and_add(&total, 1);
    }
    if (pthread_join(pthread_self(), NULL) != EDEADLK)
        return (void *)5;
    return (void *)(id + 100);
}
static void *detached_worker(void *arg) {
    (void)arg;
    while (!gate)
        sched_yield();
    __sync_fetch_and_add(&detached_done, 1);
    return NULL;
}
static void *last_thread(void *arg) {
    (void)arg;
    sleep(1);
    _exit(42);
}
static void *exec_worker(void *arg) {
    (void)arg;
    char *args[] = {"selftest", "--thread-exec-child", NULL};
    execve("/bin/selftest.elf", args, NULL);
    _exit(99);
}
static void *fork_worker(void *arg) {
    (void)arg;
    pid_t pid = fork();
    if (pid == 0)
        _exit(gettid() == getpid() ? 44 : 99);
    int status = 0;
    if (pid < 0 || waitpid(pid, &status, 0) != pid || !WIFEXITED(status) ||
        WEXITSTATUS(status) != 44)
        return (void *)1;
    return NULL;
}
void test_threads(void (*expect)(const char *, int, int)) {
    extern long __polyos_syscall6(long,long,long,long,long,long,long);
    u32 word = 1;
    long raw = __polyos_syscall6(120,0,0,0,0,0,0);
    expect("clone rejects unsupported flags",raw == -EINVAL,raw);
    raw = __polyos_syscall6(120,0x10f00,0,0,0,0,0);
    expect("clone validates child stack",raw == -EFAULT,raw);
    raw = __polyos_syscall6(240,(long)&word,0,0,0,0,0);
    expect("futex compare before sleeping",raw == -EAGAIN,raw);
    raw = __polyos_syscall6(240,0,0,0,0,0,0);
    expect("futex rejects invalid pointer",raw == -EFAULT,raw);
    pthread_t ids[8];
    expect("leader tid equals pid", gettid() == getpid(), gettid());
    expect("pthread key create", pthread_key_create(&worker_key, destroy_value) == 0, 0);
    errno = EACCES;
    int created = 0;
    for (int i = 0; i < 8; ++i) {
        int r = pthread_create(&ids[i], NULL, worker, (void *)(uintptr_t)(i + 1));
        expect("pthread create", r == 0, r);
        if (r)
            break;
        ++created;
        expect("thread id differs from pid", ids[i] != (pthread_t)getpid(), ids[i]);
        for (int j = 0; j < i; ++j)
            expect("unique thread ids", ids[i] != ids[j], ids[i]);
    }
    for (int i = 0; i < created; ++i) {
        void *result = NULL;
        int r = pthread_join(ids[i], &result);
        expect("pthread join and result", r == 0 && result == (void *)(uintptr_t)(101 + i),
               (int)(uintptr_t)result);
        expect("joined thread reaped", pthread_join(ids[i], NULL) == ESRCH, 0);
    }
    expect("shared memory across threads", total == created * 32, total);
    expect("thread local errno", errno == EACCES, errno);
    expect("TLS destructors", dtors == created, dtors);
    expect("pthread key delete", pthread_key_delete(worker_key) == 0, 0);

    pthread_attr_t attr;
    pthread_attr_init(&attr);
    expect("reject small thread stack", pthread_attr_setstacksize(&attr, 1) == EINVAL, 0);
    expect("reject unsupported guards", pthread_attr_setguardsize(&attr, 4096) == ENOTSUP, 0);
    pthread_t tid;
    gate = 0;
    int r = pthread_create(&tid, NULL, detached_worker, NULL);
    expect("detach create", r == 0, r);
    if (!r) {
        expect("pthread detach", pthread_detach(tid) == 0, 0);
        expect("cannot join detached", pthread_join(tid, NULL) == EINVAL, 0);
        gate = 1;
        while (detached_done < 1)
            sched_yield();
        sched_yield();
    }
    pthread_attr_setdetachstate(&attr, PTHREAD_CREATE_DETACHED);
    int rounds = 0;
    for (; rounds < 80; ++rounds) {
        int before = detached_done;
        r = pthread_create(&tid, &attr, detached_worker, NULL);
        if (r)
            break;
        while (detached_done == before)
            sched_yield();
        sched_yield();
    }
    expect("detached stacks and slots reclaimed", rounds == 80, rounds);

    r = pthread_create(&tid, NULL, fork_worker, NULL);
    expect("fork from worker create", r == 0, r);
    if (!r) {
        void *result = (void *)1;
        r = pthread_join(tid, &result);
        expect("fork from worker keeps one child thread", r == 0 && result == NULL, r);
    }
    pid_t pid = fork();
    if (pid == 0) {
        if (pthread_create(&tid, NULL, last_thread, NULL))
            _exit(99);
        pthread_exit(NULL);
    }
    int status = 0;
    expect("leader exit child created", pid > 0, pid);
    if (pid > 0)
        expect("leader exit preserves other threads",
               waitpid(pid, &status, 0) == pid && WIFEXITED(status) && WEXITSTATUS(status) == 42,
               status);
    pid = fork();
    if (pid == 0) {
        if (pthread_create(&tid, NULL, exec_worker, NULL))
            _exit(99);
        for (;;)
            sched_yield();
    }
    expect("worker exec child created", pid > 0, pid);
    if (pid > 0)
        expect("worker exec replaces all threads",
               waitpid(pid, &status, 0) == pid && WIFEXITED(status) && WEXITSTATUS(status) == 43,
               status);
}
