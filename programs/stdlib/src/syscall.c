#include "errno.h"
#include "polyos.h"
#include "stdio.h"
#include "types.h"

extern int __sys_execve(const char *pathname, char *const argv[], char *const envp[]);
extern int __sys_fork(void);
extern int __sys_waitpid(pid_t pid, int *status, int options);
extern int __sys_kill(pid_t pid, int sig);
extern int __sys_sigaction(int signum, const struct sigaction *act, struct sigaction *oldact);
extern void __polyos_signal_trampoline(void);
extern int __sys_nanosleep(const struct timespec *req, struct timespec *rem);
extern int __sys_gettimeofday(struct timeval *tv, struct timezone *tz);
extern int __sys_clock_gettime(clockid_t clockid, struct timespec *tp);
extern int __sys_reboot(int magic1, int magic2, int cmd, void *arg);
extern int __sys_sem_create(int initial_count);
extern int __sys_sem_wait(int semid);
extern int __sys_sem_signal(int semid);
extern int __sys_sem_close(int semid);
extern int __sys_ioctl(int fd, unsigned long request, unsigned long arg);
extern int __sys_pipe(int pipefd[2]);
extern void *__sys_brk(void *addr);

static int syscall_ret(int result)
{
    if (result < 0 && result >= -4095) {
        errno = -result;
        return -1;
    }

    return result;
}

int execve(const char *pathname, char *const argv[], char *const envp[])
{
    return syscall_ret(__sys_execve(pathname, argv, envp));
}

pid_t fork(void)
{
    extern void __heap_before_fork(void);
    extern void __pthread_before_fork(void);
    extern void __heap_parent_after_fork(void);
    extern void __pthread_parent_after_fork(void);
    __heap_before_fork();
    __pthread_before_fork();
    int result = __sys_fork();
    if (result == 0) {
        extern void __pthread_after_fork(void);
        extern void __heap_after_fork(void);
        __pthread_after_fork();
        __heap_after_fork();
    } else {
        __pthread_parent_after_fork();
        __heap_parent_after_fork();
    }
    return syscall_ret(result);
}

pid_t waitpid(pid_t pid, int *status, int options)
{
    return syscall_ret(__sys_waitpid(pid, status, options));
}

int kill(pid_t pid, int sig)
{
    return syscall_ret(__sys_kill(pid, sig));
}

int sigaction(int signum, const struct sigaction *act, struct sigaction *oldact)
{
    struct sigaction kernel_act;
    const struct sigaction *kernel_act_ptr = act;

    if (act) {
        kernel_act = *act;
        if (kernel_act.sa_handler != SIG_DFL && kernel_act.sa_handler != SIG_IGN && !kernel_act.sa_restorer) {
            kernel_act.sa_restorer = __polyos_signal_trampoline;
        }
        kernel_act_ptr = &kernel_act;
    }

    return syscall_ret(__sys_sigaction(signum, kernel_act_ptr, oldact));
}

sighandler_t signal(int signum, sighandler_t handler)
{
    struct sigaction act;
    struct sigaction oldact;

    act.sa_handler = handler;
    act.sa_flags = 0;
    act.sa_restorer = NULL;
    act.sa_mask = 0;

    if (sigaction(signum, &act, &oldact) < 0) {
        return SIG_ERR;
    }

    return oldact.sa_handler;
}

int nanosleep(const struct timespec *req, struct timespec *rem)
{
    return syscall_ret(__sys_nanosleep(req, rem));
}

int gettimeofday(struct timeval *tv, struct timezone *tz)
{
    return syscall_ret(__sys_gettimeofday(tv, tz));
}

int clock_gettime(clockid_t clockid, struct timespec *tp)
{
    return syscall_ret(__sys_clock_gettime(clockid, tp));
}

int reboot(int cmd)
{
    return syscall_ret(__sys_reboot(LINUX_REBOOT_MAGIC1, LINUX_REBOOT_MAGIC2, cmd, NULL));
}

int sem_init(sem_t *sem, int pshared, unsigned int value)
{
    if (!sem) {
        errno = EFAULT;
        return -1;
    }
    if (pshared != 0) {
        errno = ENOTSUP;
        return -1;
    }

    int id = syscall_ret(__sys_sem_create((int)value));
    if (id < 0) {
        return -1;
    }

    *sem = id;
    return 0;
}

int sem_wait(sem_t *sem)
{
    if (!sem) {
        errno = EFAULT;
        return -1;
    }

    return syscall_ret(__sys_sem_wait(*sem));
}

int sem_post(sem_t *sem)
{
    if (!sem) {
        errno = EFAULT;
        return -1;
    }

    return syscall_ret(__sys_sem_signal(*sem));
}

int sem_destroy(sem_t *sem)
{
    if (!sem) {
        errno = EFAULT;
        return -1;
    }

    int result = syscall_ret(__sys_sem_close(*sem));
    if (result == 0) {
        *sem = -1;
    }
    return result;
}

unsigned int sleep(unsigned int seconds)
{
    struct timespec req;
    req.tv_sec = seconds;
    req.tv_nsec = 0;

    return nanosleep(&req, NULL) == 0 ? 0 : seconds;
}

int ioctl(int fd, unsigned long request, unsigned long arg)
{
    return syscall_ret(__sys_ioctl(fd, request, arg));
}

int pipe(int pipefd[2])
{
    return syscall_ret(__sys_pipe(pipefd));
}

int brk(void *addr)
{
    void *result = __sys_brk(addr);
    if (result == addr) {
        return 0;
    }

    errno = ENOMEM;
    return -1;
}

void *sbrk(intptr_t increment)
{
    static char *program_break = NULL;

    if (program_break == NULL) {
        program_break = (char *)__sys_brk(NULL);
    }

    char *old_break = program_break;
    char *new_break = old_break + increment;
    char *result = (char *)__sys_brk(new_break);
    if (result != new_break) {
        errno = ENOMEM;
        return (void *)-1;
    }

    program_break = result;
    return old_break;
}
