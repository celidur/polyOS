[BITS 32]

%define SYS_EXIT 1
%define SYS_FORK 2
%define SYS_READ 3
%define SYS_WRITE 4
%define SYS_OPEN 5
%define SYS_CLOSE 6
%define SYS_WAITPID 7
%define SYS_UNLINK 10
%define SYS_EXECVE 11
%define SYS_CHDIR 12
%define SYS_LSEEK 19
%define SYS_GETPID 20
%define SYS_KILL 37
%define SYS_MKDIR 39
%define SYS_RMDIR 40
%define SYS_DUP 41
%define SYS_PIPE 42
%define SYS_BRK 45
%define SYS_IOCTL 54
%define SYS_FCNTL 55
%define SYS_DUP2 63
%define SYS_GETPPID 64
%define SYS_SIGACTION 67
%define SYS_GETTIMEOFDAY 78
%define SYS_REBOOT 88
%define SYS_SOCKETCALL 102
%define SYS_STAT 106
%define SYS_LSTAT 107
%define SYS_FSTAT 108
%define SYS_SIGRETURN 119
%define SYS_GETDENTS 141
%define SYS_NANOSLEEP 162
%define SYS_GETCWD 183
%define SYS_CLOCK_GETTIME 265

%define POLYOS_SYS_PRINT_MEMORY 503
%define POLYOS_SYS_NETWORK_INFO 520
%define POLYOS_SYS_NETWORK_DHCP_DISCOVER 521
%define POLYOS_SYS_NETWORK_PING_GATEWAY 522
%define POLYOS_SYS_NETWORK_PING_IPV4 523
%define POLYOS_SYS_NETWORK_DNS_QUERY 524
%define POLYOS_SYS_NETWORK_PING_NAME 525
%define POLYOS_SYS_RECVFROM_WAIT 529
%define POLYOS_SYS_SEM_CREATE 560
%define POLYOS_SYS_SEM_WAIT 561
%define POLYOS_SYS_SEM_SIGNAL 562
%define POLYOS_SYS_SEM_CLOSE 563
%define POLYOS_SYS_KERNEL_SELFTEST 590

section .asm

global __sys_execve:function
global __sys_fork:function
global __sys_waitpid:function
global __sys_nanosleep:function
global __sys_gettimeofday:function
global __sys_clock_gettime:function
global __sys_socketcall:function
global __sys_kill:function
global __sys_sigaction:function
global __polyos_signal_trampoline:function
global getpid:function
global getppid:function
global _exit:function
global exit:function
global print_memory:function

global __sys_reboot:function
global network_info:function
global network_dhcp_discover:function
global network_ping_gateway:function
global network_ping_ipv4:function
global network_dns_query:function
global network_ping_name:function
global __sys_recvfrom_wait:function
global __sys_close:function

global __sys_open:function
global __sys_read:function
global __sys_write:function
global __sys_lseek:function
global __sys_stat:function
global __sys_lstat:function
global __sys_fstat:function
global __sys_ioctl:function
global __sys_fcntl:function
global __sys_pipe:function
global __sys_dup:function
global __sys_dup2:function
global __sys_brk:function
global __sys_unlink:function
global __sys_mkdir:function
global __sys_rmdir:function
global __sys_chdir:function
global __sys_getcwd:function
global __sys_getdents:function
global __sys_sem_create:function
global __sys_sem_wait:function
global __sys_sem_signal:function
global __sys_sem_close:function
global kernel_selftest:function

; int __sys_execve(const char *pathname, char *const argv[], char *const envp[])
__sys_execve:
    push ebp
    mov ebp, esp
    mov eax, SYS_EXECVE
    push dword [ebp+16] ; envp
    push dword [ebp+12] ; argv
    push dword [ebp+8] ; pathname
    int 0x80
    add esp, 12
    pop ebp
    ret

; int __sys_fork()
__sys_fork:
    push ebp
    mov ebp, esp
    mov eax, SYS_FORK
    int 0x80
    pop ebp
    ret

; int __sys_kill(int pid, int sig)
__sys_kill:
    push ebp
    mov ebp, esp
    mov eax, SYS_KILL
    push dword [ebp+12] ; sig
    push dword [ebp+8] ; pid
    int 0x80
    add esp, 8
    pop ebp
    ret

; int __sys_sigaction(int signum, const struct sigaction *act, struct sigaction *oldact)
__sys_sigaction:
    push ebp
    mov ebp, esp
    mov eax, SYS_SIGACTION
    push dword [ebp+16] ; oldact
    push dword [ebp+12] ; act
    push dword [ebp+8] ; signum
    int 0x80
    add esp, 12
    pop ebp
    ret

; Returns from a userspace signal handler through sigreturn(119).
__polyos_signal_trampoline:
    mov ebx, [esp+4] ; signal frame pointer
    push ebx
    mov eax, SYS_SIGRETURN
    int 0x80
.sigreturn_failed:
    jmp .sigreturn_failed

; void exit(int code)
_exit:
exit:
    push ebp
    mov ebp, esp
    mov eax, SYS_EXIT
    push dword [ebp+8] ; code
    int 0x80
    add esp, 4
    pop ebp
    ret

; void print_memory()
print_memory:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_PRINT_MEMORY
    int 0x80
    pop ebp
    ret

; int __sys_open(const char *pathname, int flags, int mode)
__sys_open:
    push ebp
    mov ebp, esp
    mov eax, SYS_OPEN
    push dword [ebp+16] ; mode
    push dword [ebp+12] ; flags
    push dword [ebp+8] ; pathname
    int 0x80
    add esp, 12
    pop ebp
    ret

; int __sys_read(int fd, void *buf, size_t size)
__sys_read:
    push ebp
    mov ebp, esp
    mov eax, SYS_READ
    push dword [ebp+16] ; size
    push dword [ebp+12] ; buf
    push dword [ebp+8] ; fd
    int 0x80
    add esp, 12
    pop ebp
    ret

; int __sys_write(int fd, const void *buf, size_t size)
__sys_write:
    push ebp
    mov ebp, esp
    mov eax, SYS_WRITE
    push dword [ebp+16] ; size
    push dword [ebp+12] ; buf
    push dword [ebp+8] ; fd
    int 0x80
    add esp, 12
    pop ebp
    ret

; int __sys_lseek(int fd, int offset, int whence)
__sys_lseek:
    push ebp
    mov ebp, esp
    mov eax, SYS_LSEEK
    push dword [ebp+16] ; whence
    push dword [ebp+12] ; offset
    push dword [ebp+8] ; fd
    int 0x80
    add esp, 12
    pop ebp
    ret

; int __sys_fstat(int fd, struct stat *stat)
__sys_fstat:
    push ebp
    mov ebp, esp
    mov eax, SYS_FSTAT
    push dword [ebp+12] ; stat
    push dword [ebp+8] ; fd
    int 0x80
    add esp, 8
    pop ebp
    ret

; int __sys_stat(const char *pathname, struct stat *stat)
__sys_stat:
    push ebp
    mov ebp, esp
    mov eax, SYS_STAT
    push dword [ebp+12] ; stat
    push dword [ebp+8] ; pathname
    int 0x80
    add esp, 8
    pop ebp
    ret

; int __sys_lstat(const char *pathname, struct stat *stat)
__sys_lstat:
    push ebp
    mov ebp, esp
    mov eax, SYS_LSTAT
    push dword [ebp+12] ; stat
    push dword [ebp+8] ; pathname
    int 0x80
    add esp, 8
    pop ebp
    ret

; int __sys_ioctl(int fd, unsigned long request, unsigned long arg)
__sys_ioctl:
    push ebp
    mov ebp, esp
    mov eax, SYS_IOCTL
    push dword [ebp+16] ; arg
    push dword [ebp+12] ; request
    push dword [ebp+8] ; fd
    int 0x80
    add esp, 12
    pop ebp
    ret

; int __sys_fcntl(int fd, int cmd, long arg)
__sys_fcntl:
    push ebp
    mov ebp, esp
    mov eax, SYS_FCNTL
    push dword [ebp+16] ; arg
    push dword [ebp+12] ; cmd
    push dword [ebp+8] ; fd
    int 0x80
    add esp, 12
    pop ebp
    ret

; int __sys_close(int fd)
__sys_close:
    push ebp
    mov ebp, esp
    mov eax, SYS_CLOSE
    push dword [ebp+8] ; fd
    int 0x80
    add esp, 4
    pop ebp
    ret

; int __sys_reboot(int magic1, int magic2, int cmd, void *arg)
__sys_reboot:
    push ebp
    mov ebp, esp
    mov eax, SYS_REBOOT
    push dword [ebp+20] ; arg
    push dword [ebp+16] ; cmd
    push dword [ebp+12] ; magic2
    push dword [ebp+8] ; magic1
    int 0x80
    add esp, 16
    pop ebp
    ret

; int network_info(struct network_info *info)
network_info:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_NETWORK_INFO
    push dword [ebp+8] ; info
    int 0x80
    add esp, 4
    pop ebp
    ret

; int network_dhcp_discover()
network_dhcp_discover:
    mov eax, POLYOS_SYS_NETWORK_DHCP_DISCOVER
    int 0x80
    ret

; int network_ping_gateway()
network_ping_gateway:
    mov eax, POLYOS_SYS_NETWORK_PING_GATEWAY
    int 0x80
    ret

; int network_ping_ipv4(u32 ip)
network_ping_ipv4:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_NETWORK_PING_IPV4
    push dword [ebp+8] ; ip
    int 0x80
    add esp, 4
    pop ebp
    ret

; int network_dns_query(const char *name)
network_dns_query:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_NETWORK_DNS_QUERY
    push dword [ebp+8] ; name
    int 0x80
    add esp, 4
    pop ebp
    ret

; int network_ping_name(const char *name)
network_ping_name:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_NETWORK_PING_NAME
    push dword [ebp+8] ; name
    int 0x80
    add esp, 4
    pop ebp
    ret

; int __sys_socketcall(int call, unsigned long *args)
__sys_socketcall:
    push ebp
    mov ebp, esp
    mov eax, SYS_SOCKETCALL
    push dword [ebp+12] ; args
    push dword [ebp+8] ; call
    int 0x80
    add esp, 8
    pop ebp
    ret

; int __sys_recvfrom_wait(int sockfd, void *buf, size_t len, int flags, struct sockaddr *src_addr, socklen_t *addrlen, u32 timeout_ticks)
__sys_recvfrom_wait:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_RECVFROM_WAIT
    push dword [ebp+32] ; timeout_ticks
    push dword [ebp+28] ; addrlen
    push dword [ebp+24] ; src_addr
    push dword [ebp+20] ; flags
    push dword [ebp+16] ; len
    push dword [ebp+12] ; buf
    push dword [ebp+8] ; sockfd
    int 0x80
    add esp, 28
    pop ebp
    ret

; int __sys_waitpid(int pid, int *status, int options)
__sys_waitpid:
    push ebp
    mov ebp, esp
    mov eax, SYS_WAITPID
    push dword [ebp+16] ; options
    push dword [ebp+12] ; status
    push dword [ebp+8] ; pid
    int 0x80
    add esp, 12
    pop ebp
    ret

; int __sys_nanosleep(const struct timespec *req, struct timespec *rem)
__sys_nanosleep:
    push ebp
    mov ebp, esp
    mov eax, SYS_NANOSLEEP
    push dword [ebp+12] ; rem
    push dword [ebp+8] ; req
    int 0x80
    add esp, 8
    pop ebp
    ret

; int __sys_gettimeofday(struct timeval *tv, struct timezone *tz)
__sys_gettimeofday:
    push ebp
    mov ebp, esp
    mov eax, SYS_GETTIMEOFDAY
    push dword [ebp+12] ; tz
    push dword [ebp+8] ; tv
    int 0x80
    add esp, 8
    pop ebp
    ret

; int __sys_clock_gettime(int clockid, struct timespec *tp)
__sys_clock_gettime:
    push ebp
    mov ebp, esp
    mov eax, SYS_CLOCK_GETTIME
    push dword [ebp+12] ; tp
    push dword [ebp+8] ; clockid
    int 0x80
    add esp, 8
    pop ebp
    ret

; int getpid()
getpid:
    mov eax, SYS_GETPID
    int 0x80
    ret

; int getppid()
getppid:
    mov eax, SYS_GETPPID
    int 0x80
    ret

; int __sys_pipe(int pipefd[2])
__sys_pipe:
    push ebp
    mov ebp, esp
    mov eax, SYS_PIPE
    push dword [ebp+8] ; pipefd
    int 0x80
    add esp, 4
    pop ebp
    ret

; int __sys_dup(int oldfd)
__sys_dup:
    push ebp
    mov ebp, esp
    mov eax, SYS_DUP
    push dword [ebp+8] ; oldfd
    int 0x80
    add esp, 4
    pop ebp
    ret

; int __sys_dup2(int oldfd, int newfd)
__sys_dup2:
    push ebp
    mov ebp, esp
    mov eax, SYS_DUP2
    push dword [ebp+12] ; newfd
    push dword [ebp+8] ; oldfd
    int 0x80
    add esp, 8
    pop ebp
    ret

; void *__sys_brk(void *addr)
__sys_brk:
    push ebp
    mov ebp, esp
    mov eax, SYS_BRK
    push dword [ebp+8] ; addr
    int 0x80
    add esp, 4
    pop ebp
    ret

; int __sys_unlink(const char *pathname)
__sys_unlink:
    push ebp
    mov ebp, esp
    mov eax, SYS_UNLINK
    push dword [ebp+8] ; pathname
    int 0x80
    add esp, 4
    pop ebp
    ret

; int __sys_mkdir(const char *pathname, int mode)
__sys_mkdir:
    push ebp
    mov ebp, esp
    mov eax, SYS_MKDIR
    push dword [ebp+12] ; mode
    push dword [ebp+8] ; pathname
    int 0x80
    add esp, 8
    pop ebp
    ret

; int __sys_rmdir(const char *pathname)
__sys_rmdir:
    push ebp
    mov ebp, esp
    mov eax, SYS_RMDIR
    push dword [ebp+8] ; pathname
    int 0x80
    add esp, 4
    pop ebp
    ret

; int __sys_chdir(const char *pathname)
__sys_chdir:
    push ebp
    mov ebp, esp
    mov eax, SYS_CHDIR
    push dword [ebp+8] ; pathname
    int 0x80
    add esp, 4
    pop ebp
    ret

; int __sys_getcwd(char *buf, size_t size)
__sys_getcwd:
    push ebp
    mov ebp, esp
    mov eax, SYS_GETCWD
    push dword [ebp+12] ; size
    push dword [ebp+8] ; buf
    int 0x80
    add esp, 8
    pop ebp
    ret

; int __sys_getdents(int fd, struct dirent *dirp, size_t count)
__sys_getdents:
    push ebp
    mov ebp, esp
    mov eax, SYS_GETDENTS
    push dword [ebp+16] ; count
    push dword [ebp+12] ; dirp
    push dword [ebp+8] ; fd
    int 0x80
    add esp, 12
    pop ebp
    ret

; int __sys_sem_create(int initial_count)
__sys_sem_create:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_SEM_CREATE
    push dword [ebp+8] ; initial_count
    int 0x80
    add esp, 4
    pop ebp
    ret

; int __sys_sem_wait(int semid)
__sys_sem_wait:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_SEM_WAIT
    push dword [ebp+8] ; semid
    int 0x80
    add esp, 4
    pop ebp
    ret

; int __sys_sem_signal(int semid)
__sys_sem_signal:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_SEM_SIGNAL
    push dword [ebp+8] ; semid
    int 0x80
    add esp, 4
    pop ebp
    ret

; int __sys_sem_close(int semid)
__sys_sem_close:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_SEM_CLOSE
    push dword [ebp+8] ; semid
    int 0x80
    add esp, 4
    pop ebp
    ret

; int kernel_selftest()
kernel_selftest:
    mov eax, POLYOS_SYS_KERNEL_SELFTEST
    int 0x80
    ret
