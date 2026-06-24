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
