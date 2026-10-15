#ifndef POLYOS_H
#define POLYOS_H

#include <types.h>
#include <stdbool.h>

struct network_info {
    u32 present;
    u32 dhcp_state;
    u8 mac[6];
    u8 _padding[2];
    u8 ipv4[4];
    u8 subnet_mask[4];
    u8 router[4];
    u8 dns[4];
    u64 packets_rx;
    u64 packets_tx;
    u32 arp_entries;
    u32 ping_tx;
    u32 ping_rx;
    u32 dns_tx;
    u32 dns_rx;
};

#define AF_UNIX 1
#define SOCK_STREAM 1
#define SOCK_NONBLOCK 0x800
#define SOCK_CLOEXEC 0x80000
#define O_CLOEXEC 0x80000
#define AF_INET 2
#define SOCK_DGRAM 2
#define SOCK_RAW 3
#define IPPROTO_ICMP 1
#define IPPROTO_UDP 17
#define SOL_SOCKET 1
#define SO_REUSEADDR 2
#define WNOHANG 1
#define WIFEXITED(status) (((status) & 0x7f) == 0)
#define WEXITSTATUS(status) (((status) >> 8) & 0xff)
#define WIFSIGNALED(status) ((((status) & 0x7f) != 0) && (((status) & 0x7f) != 0x7f))
#define WTERMSIG(status) ((status) & 0x7f)
#define SIG_DFL ((sighandler_t)0)
#define SIG_IGN ((sighandler_t)1)
#define SIG_ERR ((sighandler_t)-1)
#define SIGHUP 1
#define SIGINT 2
#define SIGQUIT 3
#define SIGKILL 9
#define SIGUSR1 10
#define SIGSEGV 11
#define SIGUSR2 12
#define SIGPIPE 13
#define SIGALRM 14
#define SIGTERM 15
#define SIGCHLD 17
#define SIGCONT 18
#define SIGSTOP 19
#define LINUX_REBOOT_MAGIC1 0xfee1dead
#define LINUX_REBOOT_MAGIC2 672274793
#define RB_AUTOBOOT 0x01234567
#define RB_HALT_SYSTEM 0xcdef0123
#define RB_POWER_OFF 0x4321fedc

#define htons(x) ((((x) & 0xff) << 8) | (((x) >> 8) & 0xff))
#define htonl(x) ((((x) & 0xff) << 24) | (((x) & 0xff00) << 8) | (((x) >> 8) & 0xff00) | (((x) >> 24) & 0xff))

typedef u32 socklen_t;
typedef u32 sigset_t;
typedef void (*sighandler_t)(int);
typedef int sem_t;

struct sigaction {
    sighandler_t sa_handler;
    u32 sa_flags;
    void (*sa_restorer)(void);
    sigset_t sa_mask;
};

struct in_addr {
    u32 s_addr;
};

struct sockaddr {
    u16 sa_family;
    char sa_data[14];
};

struct sockaddr_in {
    u16 sin_family;
    u16 sin_port;
    struct in_addr sin_addr;
    u8 sin_zero[8];
};

void polyos_terminal_readline(char* out, int max, bool output_while_typing);
int polyos_system_run(const char *command);
#define AT_FDCWD (-100)
#define AT_SYMLINK_NOFOLLOW 0x100
#define AT_EMPTY_PATH 0x1000
#define O_EXCL 0x80
#define O_DIRECTORY 0x10000
#define F_OK 0
#define X_OK 1
#define W_OK 2
#define R_OK 4
#define S_IFLNK 0120000
#define S_ISLNK(mode) (((mode) & S_IFMT) == S_IFLNK)
struct stat64 {
    u64 st_dev; u32 __pad0; u32 __st_ino;
    u32 st_mode, st_nlink, st_uid, st_gid;
    u64 st_rdev; u32 __pad1; s64 st_size; u32 st_blksize; u64 st_blocks;
    u32 st_atime, st_atime_nsec, st_mtime, st_mtime_nsec, st_ctime, st_ctime_nsec;
    u64 st_ino;
} __attribute__((packed));
struct dirent64 { u64 d_ino; s64 d_off; u16 d_reclen; u8 d_type; char d_name[]; } __attribute__((packed));
int openat(int dirfd, const char *path, int flags, int mode);
int ftruncate(int fd, off_t length);
int fchmod(int fd, int mode);
int access(const char *path, int mode);
ssize_t readlink(const char *path, char *buf, size_t size);
int fstatat(int dirfd, const char *path, struct file_stat *out, int flags);
int fstatat64(int dirfd, const char *path, struct stat64 *out, int flags);
int stat64(const char *path, struct stat64 *out);
int lstat64(const char *path, struct stat64 *out);
int fstat64(int fd, struct stat64 *out);
int getdents64(int fd, void *buffer, size_t size);
int open(const char *pathname, int flags, int mode);
ssize_t read(int fd, void *buf, size_t count);
ssize_t write(int fd, const void *buf, size_t count);
off_t lseek(int fd, off_t offset, int whence);
int stat(const char *pathname, struct file_stat *stat);
int lstat(const char *pathname, struct file_stat *stat);
int ioctl(int fd, unsigned long request, unsigned long arg);
int fcntl(int fd, int cmd, long arg);
int close(int fd);
int pipe(int pipefd[2]);
int pipe2(int pipefd[2], int flags);
int socketpair(int domain, int type, int protocol, int sv[2]);
struct iovec { void *iov_base; size_t iov_len; };
ssize_t writev(int fd, const struct iovec *iov, int count);
#define POLLIN 1
#define POLLOUT 4
#define POLLERR 8
#define POLLHUP 16
#define POLLNVAL 32
struct pollfd { int fd; short events; short revents; };
int poll(struct pollfd *fds, unsigned int count, int timeout);
#define FD_SETSIZE 128
typedef struct { u32 bits[FD_SETSIZE / 32]; } fd_set;
#define FD_ZERO(set) do { for (unsigned _i = 0; _i < FD_SETSIZE/32; ++_i) (set)->bits[_i] = 0; } while (0)
#define FD_SET(fd,set) ((set)->bits[(fd)/32] |= 1u << ((fd)%32))
#define FD_CLR(fd,set) ((set)->bits[(fd)/32] &= ~(1u << ((fd)%32)))
#define FD_ISSET(fd,set) (((set)->bits[(fd)/32] >> ((fd)%32)) & 1)
int select(int count, fd_set *readfds, fd_set *writefds, fd_set *exceptfds, struct timeval *timeout);
int dup(int oldfd);
int dup2(int oldfd, int newfd);
int brk(void *addr);
void *sbrk(intptr_t increment);
int unlink(const char *pathname);
int chmod(const char *pathname, int mode);
int mkdir(const char *pathname, int mode);
int rmdir(const char *pathname);
int umask(int mask);
int chdir(const char *pathname);
int chown(const char *pathname, unsigned int uid, unsigned int gid);
char *getcwd(char *buf, size_t size);
int getdents(int fd, struct dirent *dirp, size_t count);
int sem_init(sem_t *sem, int pshared, unsigned int value);
int sem_wait(sem_t *sem);
int sem_post(sem_t *sem);
int sem_destroy(sem_t *sem);
int kernel_selftest();
int execve(const char *pathname, char *const argv[], char *const envp[]);
pid_t fork();
pid_t waitpid(pid_t pid, int *status, int options);
pid_t getpid();
pid_t gettid(void);
int sched_yield(void);
pid_t getppid();
uid_t getuid();
gid_t getgid();
uid_t geteuid();
gid_t getegid();
int kill(pid_t pid, int sig);
int sigaction(int signum, const struct sigaction *act, struct sigaction *oldact);
sighandler_t signal(int signum, sighandler_t handler);
int nanosleep(const struct timespec *req, struct timespec *rem);
int gettimeofday(struct timeval *tv, struct timezone *tz);
int clock_gettime(clockid_t clockid, struct timespec *tp);
unsigned int sleep(unsigned int seconds);
void _exit(int code) __attribute__((noreturn));
void exit(int code) __attribute__((noreturn));
void print_memory();
int clear_screen();
int reboot(int cmd);
int gethostbyname_ipv4(const char *name, u32 *out_addr);
int network_info(struct network_info *info);
int network_dhcp_discover();
int network_ping_gateway();
int network_ping_ipv4(u32 ip);
int network_dns_query(const char *name);
int network_ping_name(const char *name);
int socket(int domain, int type, int protocol);
int sendto(int sockfd, const void *buf, size_t len, int flags, const struct sockaddr *dest_addr, socklen_t addrlen);
int recvfrom(int sockfd, void *buf, size_t len, int flags, struct sockaddr *src_addr, socklen_t *addrlen);
int recvfrom_wait(int sockfd, void *buf, size_t len, int flags, struct sockaddr *src_addr, socklen_t *addrlen, u32 timeout_ticks);
int bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen);
int connect(int sockfd, const struct sockaddr *addr, socklen_t addrlen);
int listen(int sockfd, int backlog);
int accept(int sockfd, struct sockaddr *addr, socklen_t *addrlen);
ssize_t send(int sockfd, const void *buf, size_t len, int flags);
ssize_t recv(int sockfd, void *buf, size_t len, int flags);
int getsockname(int sockfd, struct sockaddr *addr, socklen_t *addrlen);
int getpeername(int sockfd, struct sockaddr *addr, socklen_t *addrlen);
int setsockopt(int sockfd, int level, int optname, const void *optval, socklen_t optlen);

#endif
