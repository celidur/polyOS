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
extern int __sys_socketcall(int call, unsigned long *args);
extern int __sys_recvfrom_wait(int sockfd, void *buf, size_t len, int flags, struct sockaddr *src_addr, socklen_t *addrlen, u32 timeout_ticks);
extern int __sys_sem_create(int initial_count);
extern int __sys_sem_wait(int semid);
extern int __sys_sem_signal(int semid);
extern int __sys_sem_close(int semid);
extern int __sys_open(const char *pathname, int flags, int mode);
extern ssize_t __sys_read(int fd, void *buf, size_t count);
extern ssize_t __sys_write(int fd, const void *buf, size_t count);
extern off_t __sys_lseek(int fd, off_t offset, int whence);
extern int __sys_stat(const char *pathname, struct file_stat *stat);
extern int __sys_lstat(const char *pathname, struct file_stat *stat);
extern int __sys_fstat(int fd, struct file_stat *stat);
extern int __sys_ioctl(int fd, unsigned long request, unsigned long arg);
extern int __sys_fcntl(int fd, int cmd, long arg);
extern int __sys_close(int fd);
extern int __sys_pipe(int pipefd[2]);
extern int __sys_dup(int oldfd);
extern int __sys_dup2(int oldfd, int newfd);
extern void *__sys_brk(void *addr);
extern int __sys_unlink(const char *pathname);
extern int __sys_chmod(const char *pathname, int mode);
extern int __sys_mkdir(const char *pathname, int mode);
extern int __sys_rmdir(const char *pathname);
extern int __sys_umask(int mask);
extern int __sys_chdir(const char *pathname);
extern int __sys_chown(const char *pathname, unsigned int uid, unsigned int gid);
extern int __sys_getcwd(char *buf, size_t size);
extern int __sys_getdents(int fd, struct dirent *dirp, size_t count);

static int syscall_ret(int result)
{
    if (result < 0 && result >= -4095) {
        errno = -result;
        return -1;
    }

    return result;
}

enum
{
    SOCKETCALL_SOCKET = 1,
    SOCKETCALL_BIND = 2,
    SOCKETCALL_CONNECT = 3,
    SOCKETCALL_LISTEN = 4,
    SOCKETCALL_ACCEPT = 5,
    SOCKETCALL_GETSOCKNAME = 6,
    SOCKETCALL_GETPEERNAME = 7,
    SOCKETCALL_SEND = 9,
    SOCKETCALL_RECV = 10,
    SOCKETCALL_SENDTO = 11,
    SOCKETCALL_RECVFROM = 12,
    SOCKETCALL_SETSOCKOPT = 14,
};

int execve(const char *pathname, char *const argv[], char *const envp[])
{
    return syscall_ret(__sys_execve(pathname, argv, envp));
}

pid_t fork(void)
{
    return syscall_ret(__sys_fork());
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

int socket(int domain, int type, int protocol)
{
    unsigned long args[] = {
        (unsigned long)domain,
        (unsigned long)type,
        (unsigned long)protocol,
    };
    return syscall_ret(__sys_socketcall(SOCKETCALL_SOCKET, args));
}

int sendto(int sockfd, const void *buf, size_t len, int flags, const struct sockaddr *dest_addr, socklen_t addrlen)
{
    unsigned long args[] = {
        (unsigned long)sockfd,
        (unsigned long)buf,
        (unsigned long)len,
        (unsigned long)flags,
        (unsigned long)dest_addr,
        (unsigned long)addrlen,
    };
    return syscall_ret(__sys_socketcall(SOCKETCALL_SENDTO, args));
}

int recvfrom(int sockfd, void *buf, size_t len, int flags, struct sockaddr *src_addr, socklen_t *addrlen)
{
    unsigned long args[] = {
        (unsigned long)sockfd,
        (unsigned long)buf,
        (unsigned long)len,
        (unsigned long)flags,
        (unsigned long)src_addr,
        (unsigned long)addrlen,
    };
    return syscall_ret(__sys_socketcall(SOCKETCALL_RECVFROM, args));
}

int recvfrom_wait(int sockfd, void *buf, size_t len, int flags, struct sockaddr *src_addr, socklen_t *addrlen, u32 timeout_ticks)
{
    return syscall_ret(__sys_recvfrom_wait(sockfd, buf, len, flags, src_addr, addrlen, timeout_ticks));
}

int bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen)
{
    unsigned long args[] = {
        (unsigned long)sockfd,
        (unsigned long)addr,
        (unsigned long)addrlen,
    };
    return syscall_ret(__sys_socketcall(SOCKETCALL_BIND, args));
}

int connect(int sockfd, const struct sockaddr *addr, socklen_t addrlen)
{
    unsigned long args[] = {
        (unsigned long)sockfd,
        (unsigned long)addr,
        (unsigned long)addrlen,
    };
    return syscall_ret(__sys_socketcall(SOCKETCALL_CONNECT, args));
}

int listen(int sockfd, int backlog)
{
    unsigned long args[] = {
        (unsigned long)sockfd,
        (unsigned long)backlog,
    };
    return syscall_ret(__sys_socketcall(SOCKETCALL_LISTEN, args));
}

int accept(int sockfd, struct sockaddr *addr, socklen_t *addrlen)
{
    unsigned long args[] = {
        (unsigned long)sockfd,
        (unsigned long)addr,
        (unsigned long)addrlen,
    };
    return syscall_ret(__sys_socketcall(SOCKETCALL_ACCEPT, args));
}

ssize_t send(int sockfd, const void *buf, size_t len, int flags)
{
    unsigned long args[] = {
        (unsigned long)sockfd,
        (unsigned long)buf,
        (unsigned long)len,
        (unsigned long)flags,
    };
    return syscall_ret(__sys_socketcall(SOCKETCALL_SEND, args));
}

ssize_t recv(int sockfd, void *buf, size_t len, int flags)
{
    unsigned long args[] = {
        (unsigned long)sockfd,
        (unsigned long)buf,
        (unsigned long)len,
        (unsigned long)flags,
    };
    return syscall_ret(__sys_socketcall(SOCKETCALL_RECV, args));
}

int getsockname(int sockfd, struct sockaddr *addr, socklen_t *addrlen)
{
    unsigned long args[] = {
        (unsigned long)sockfd,
        (unsigned long)addr,
        (unsigned long)addrlen,
    };
    return syscall_ret(__sys_socketcall(SOCKETCALL_GETSOCKNAME, args));
}

int getpeername(int sockfd, struct sockaddr *addr, socklen_t *addrlen)
{
    unsigned long args[] = {
        (unsigned long)sockfd,
        (unsigned long)addr,
        (unsigned long)addrlen,
    };
    return syscall_ret(__sys_socketcall(SOCKETCALL_GETPEERNAME, args));
}

int setsockopt(int sockfd, int level, int optname, const void *optval, socklen_t optlen)
{
    unsigned long args[] = {
        (unsigned long)sockfd,
        (unsigned long)level,
        (unsigned long)optname,
        (unsigned long)optval,
        (unsigned long)optlen,
    };
    return syscall_ret(__sys_socketcall(SOCKETCALL_SETSOCKOPT, args));
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

int open(const char *pathname, int flags, int mode)
{
    return syscall_ret(__sys_open(pathname, flags, mode));
}

ssize_t read(int fd, void *buf, size_t count)
{
    return syscall_ret(__sys_read(fd, buf, count));
}

ssize_t write(int fd, const void *buf, size_t count)
{
    return syscall_ret(__sys_write(fd, buf, count));
}

off_t lseek(int fd, off_t offset, int whence)
{
    return syscall_ret(__sys_lseek(fd, offset, whence));
}

int fstat(int fd, struct file_stat *stat)
{
    return syscall_ret(__sys_fstat(fd, stat));
}

int stat(const char *pathname, struct file_stat *stat)
{
    return syscall_ret(__sys_stat(pathname, stat));
}

int lstat(const char *pathname, struct file_stat *stat)
{
    return syscall_ret(__sys_lstat(pathname, stat));
}

int ioctl(int fd, unsigned long request, unsigned long arg)
{
    return syscall_ret(__sys_ioctl(fd, request, arg));
}

int fcntl(int fd, int cmd, long arg)
{
    return syscall_ret(__sys_fcntl(fd, cmd, arg));
}

int close(int fd)
{
    return syscall_ret(__sys_close(fd));
}

int pipe(int pipefd[2])
{
    return syscall_ret(__sys_pipe(pipefd));
}

int dup(int oldfd)
{
    return syscall_ret(__sys_dup(oldfd));
}

int dup2(int oldfd, int newfd)
{
    return syscall_ret(__sys_dup2(oldfd, newfd));
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

int unlink(const char *pathname)
{
    return syscall_ret(__sys_unlink(pathname));
}

int chmod(const char *pathname, int mode)
{
    return syscall_ret(__sys_chmod(pathname, mode));
}

int mkdir(const char *pathname, int mode)
{
    return syscall_ret(__sys_mkdir(pathname, mode));
}

int rmdir(const char *pathname)
{
    return syscall_ret(__sys_rmdir(pathname));
}

int umask(int mask)
{
    return __sys_umask(mask);
}

int chdir(const char *pathname)
{
    return syscall_ret(__sys_chdir(pathname));
}

int chown(const char *pathname, unsigned int uid, unsigned int gid)
{
    return syscall_ret(__sys_chown(pathname, uid, gid));
}

char *getcwd(char *buf, size_t size)
{
    int result = __sys_getcwd(buf, size);
    if (result < 0 && result >= -4095) {
        errno = -result;
        return NULL;
    }

    return buf;
}

int getdents(int fd, struct dirent *dirp, size_t count)
{
    return syscall_ret(__sys_getdents(fd, dirp, count));
}
