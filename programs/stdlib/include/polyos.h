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
#define POLYOS_WAIT_TIMEOUT -2

#define htons(x) ((((x) & 0xff) << 8) | (((x) >> 8) & 0xff))
#define htonl(x) ((((x) & 0xff) << 24) | (((x) & 0xff00) << 8) | (((x) >> 8) & 0xff00) | (((x) >> 24) & 0xff))

typedef u32 socklen_t;

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

void polyos_sleep(u32 duration_ms);
void polyos_terminal_readline(char* out, int max, bool output_while_typing);
void* polyos_malloc(size_t size);
void polyos_free(void* ptr);
int polyos_system_run(const char *command);
int open(const char *pathname, int flags, int mode);
ssize_t read(int fd, void *buf, size_t count);
ssize_t write(int fd, const void *buf, size_t count);
off_t lseek(int fd, off_t offset, int whence);
int ioctl(int fd, unsigned long request, unsigned long arg);
int close(int fd);
int pipe(int pipefd[2]);
int sem_create(int initial_count);
int sem_wait(int semid);
int sem_signal(int semid);
int sem_close(int semid);
int kernel_selftest();
int execve(const char *pathname, char *const argv[], char *const envp[]);
pid_t fork();
pid_t waitpid(pid_t pid, int *status, int options);
pid_t getpid();
pid_t getppid();
void _exit(int code) __attribute__((noreturn));
void exit(int code) __attribute__((noreturn));
void print_memory();
int clear_screen();
void reboot();
void shutdown();
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

#endif
