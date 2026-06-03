[BITS 32]

%define SYS_EXIT 1
%define SYS_FORK 2
%define SYS_READ 3
%define SYS_WRITE 4
%define SYS_OPEN 5
%define SYS_CLOSE 6
%define SYS_WAITPID 7
%define SYS_EXECVE 11
%define SYS_LSEEK 19
%define SYS_GETPID 20
%define SYS_PIPE 42
%define SYS_IOCTL 54
%define SYS_GETPPID 64
%define SYS_FSTAT 108

%define POLYOS_SYS_SLEEP 200
%define POLYOS_SYS_MALLOC 201
%define POLYOS_SYS_FREE 202
%define POLYOS_SYS_PRINT_MEMORY 203
%define POLYOS_SYS_REBOOT 204
%define POLYOS_SYS_SHUTDOWN 205
%define POLYOS_SYS_NETWORK_INFO 220
%define POLYOS_SYS_NETWORK_DHCP_DISCOVER 221
%define POLYOS_SYS_NETWORK_PING_GATEWAY 222
%define POLYOS_SYS_NETWORK_PING_IPV4 223
%define POLYOS_SYS_NETWORK_DNS_QUERY 224
%define POLYOS_SYS_NETWORK_PING_NAME 225
%define POLYOS_SYS_SOCKET 226
%define POLYOS_SYS_SENDTO 227
%define POLYOS_SYS_RECVFROM 228
%define POLYOS_SYS_SEM_CREATE 230
%define POLYOS_SYS_SEM_WAIT 231
%define POLYOS_SYS_SEM_SIGNAL 232
%define POLYOS_SYS_SEM_CLOSE 233
%define POLYOS_SYS_KERNEL_SELFTEST 234

section .asm

global polyos_sleep:function
global polyos_malloc:function
global polyos_free:function
global execve:function
global fork:function
global waitpid:function
global getpid:function
global getppid:function
global _exit:function
global exit:function
global print_memory:function

global reboot:function
global shutdown:function
global network_info:function
global network_dhcp_discover:function
global network_ping_gateway:function
global network_ping_ipv4:function
global network_dns_query:function
global network_ping_name:function
global socket:function
global sendto:function
global recvfrom:function
global close:function

global open:function
global read:function
global write:function
global lseek:function
global fstat:function
global ioctl:function
global pipe:function
global sem_create:function
global sem_wait:function
global sem_signal:function
global sem_close:function
global kernel_selftest:function

; void polyos_sleep(u32 duration_ms)
polyos_sleep:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_SLEEP
    push dword [ebp+8] ; duration_ms
    int 0x80
    add esp, 4
    pop ebp
    ret

; void* polyos_malloc(size_t size)
polyos_malloc:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_MALLOC
    push dword [ebp+8] ; size
    int 0x80
    add esp, 4
    pop ebp
    ret

; void polyos_free(void *ptr)
polyos_free:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_FREE
    push dword [ebp+8] ; ptr
    int 0x80
    add esp, 4
    pop ebp
    ret

; int execve(const char *pathname, char *const argv[], char *const envp[])
execve:
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

; int fork()
fork:
    push ebp
    mov ebp, esp
    mov eax, SYS_FORK
    int 0x80
    pop ebp
    ret

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

; int open(const char *pathname, int flags, int mode)
open:
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

; int read(int fd, void *buf, size_t size)
read:
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

; int write(int fd, const void *buf, size_t size)
write:
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

; int lseek(int fd, int offset, int whence)
lseek:
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

; int fstat(int fd, struct stat *stat)
fstat:
    push ebp
    mov ebp, esp
    mov eax, SYS_FSTAT
    push dword [ebp+12] ; stat
    push dword [ebp+8] ; fd
    int 0x80
    add esp, 8
    pop ebp
    ret

; int ioctl(int fd, unsigned long request, unsigned long arg)
ioctl:
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

; int close(int fd)
close:
    push ebp
    mov ebp, esp
    mov eax, SYS_CLOSE
    push dword [ebp+8] ; fd
    int 0x80
    add esp, 4
    pop ebp
    ret

; void reboot()
reboot:
    mov eax, POLYOS_SYS_REBOOT
    int 0x80
    ret

; void shutdown()
shutdown:
    mov eax, POLYOS_SYS_SHUTDOWN
    int 0x80
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

; int socket(int domain, int type, int protocol)
socket:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_SOCKET
    push dword [ebp+16] ; protocol
    push dword [ebp+12] ; type
    push dword [ebp+8] ; domain
    int 0x80
    add esp, 12
    pop ebp
    ret

; int sendto(int sockfd, const void *buf, size_t len, int flags, const struct sockaddr *dest_addr, socklen_t addrlen)
sendto:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_SENDTO
    push dword [ebp+28] ; addrlen
    push dword [ebp+24] ; dest_addr
    push dword [ebp+20] ; flags
    push dword [ebp+16] ; len
    push dword [ebp+12] ; buf
    push dword [ebp+8] ; sockfd
    int 0x80
    add esp, 24
    pop ebp
    ret

; int recvfrom(int sockfd, void *buf, size_t len, int flags, struct sockaddr *src_addr, socklen_t *addrlen)
recvfrom:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_RECVFROM
    push dword [ebp+28] ; addrlen
    push dword [ebp+24] ; src_addr
    push dword [ebp+20] ; flags
    push dword [ebp+16] ; len
    push dword [ebp+12] ; buf
    push dword [ebp+8] ; sockfd
    int 0x80
    add esp, 24
    pop ebp
    ret

; int waitpid(int pid)
waitpid:
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

; int pipe(int pipefd[2])
pipe:
    push ebp
    mov ebp, esp
    mov eax, SYS_PIPE
    push dword [ebp+8] ; pipefd
    int 0x80
    add esp, 4
    pop ebp
    ret

; int sem_create(int initial_count)
sem_create:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_SEM_CREATE
    push dword [ebp+8] ; initial_count
    int 0x80
    add esp, 4
    pop ebp
    ret

; int sem_wait(int semid)
sem_wait:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_SEM_WAIT
    push dword [ebp+8] ; semid
    int 0x80
    add esp, 4
    pop ebp
    ret

; int sem_signal(int semid)
sem_signal:
    push ebp
    mov ebp, esp
    mov eax, POLYOS_SYS_SEM_SIGNAL
    push dword [ebp+8] ; semid
    int 0x80
    add esp, 4
    pop ebp
    ret

; int sem_close(int semid)
sem_close:
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
