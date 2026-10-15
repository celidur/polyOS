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
%define SYS_CHMOD 15
%define SYS_LSEEK 19
%define SYS_GETPID 20
%define SYS_GETUID 24
%define SYS_KILL 37
%define SYS_MKDIR 39
%define SYS_RMDIR 40
%define SYS_DUP 41
%define SYS_PIPE 42
%define SYS_BRK 45
%define SYS_GETGID 47
%define SYS_GETEUID 49
%define SYS_GETEGID 50
%define SYS_IOCTL 54
%define SYS_FCNTL 55
%define SYS_UMASK 60
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
%define SYS_CHOWN 182
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

section .text

; Preserve i386 cdecl callee-saved registers. Linux i386 int 0x80 register ABI.
%macro SYSCALL 3
    global %1:function
%1:
    push ebp
    mov ebp, esp
    push ebx
    push esi
    push edi
    mov eax, %2
%if %3 > 0
    mov ebx, [ebp+8]
%endif
%if %3 > 1
    mov ecx, [ebp+12]
%endif
%if %3 > 2
    mov edx, [ebp+16]
%endif
%if %3 > 3
    mov esi, [ebp+20]
%endif
%if %3 > 4
    mov edi, [ebp+24]
%endif
%if %3 > 5
    mov ebp, [ebp+28]
%endif
    int 0x80
    pop edi
    pop esi
    pop ebx
    pop ebp
    ret
%endmacro

SYSCALL __sys_execve, SYS_EXECVE, 3
SYSCALL __sys_fork, SYS_FORK, 0
SYSCALL __sys_kill, SYS_KILL, 2
SYSCALL __sys_sigaction, SYS_SIGACTION, 3
SYSCALL exit, SYS_EXIT, 1
SYSCALL print_memory, POLYOS_SYS_PRINT_MEMORY, 0
SYSCALL __sys_open, SYS_OPEN, 3
SYSCALL __sys_read, SYS_READ, 3
SYSCALL __sys_write, SYS_WRITE, 3
SYSCALL __sys_lseek, SYS_LSEEK, 3
SYSCALL __sys_fstat, SYS_FSTAT, 2
SYSCALL __sys_stat, SYS_STAT, 2
SYSCALL __sys_lstat, SYS_LSTAT, 2
SYSCALL __sys_ioctl, SYS_IOCTL, 3
SYSCALL __sys_fcntl, SYS_FCNTL, 3
SYSCALL __sys_close, SYS_CLOSE, 1
SYSCALL __sys_reboot, SYS_REBOOT, 4
SYSCALL network_info, POLYOS_SYS_NETWORK_INFO, 1
SYSCALL network_dhcp_discover, POLYOS_SYS_NETWORK_DHCP_DISCOVER, 0
SYSCALL network_ping_gateway, POLYOS_SYS_NETWORK_PING_GATEWAY, 0
SYSCALL network_ping_ipv4, POLYOS_SYS_NETWORK_PING_IPV4, 1
SYSCALL network_dns_query, POLYOS_SYS_NETWORK_DNS_QUERY, 1
SYSCALL network_ping_name, POLYOS_SYS_NETWORK_PING_NAME, 1
SYSCALL __sys_socketcall, SYS_SOCKETCALL, 2
SYSCALL __sys_recvfrom_wait, POLYOS_SYS_RECVFROM_WAIT, 1
SYSCALL __sys_waitpid, SYS_WAITPID, 3
SYSCALL __sys_nanosleep, SYS_NANOSLEEP, 2
SYSCALL __sys_gettimeofday, SYS_GETTIMEOFDAY, 2
SYSCALL __sys_clock_gettime, SYS_CLOCK_GETTIME, 2
SYSCALL getpid, SYS_GETPID, 0
SYSCALL getuid, SYS_GETUID, 0
SYSCALL getppid, SYS_GETPPID, 0
SYSCALL getgid, SYS_GETGID, 0
SYSCALL geteuid, SYS_GETEUID, 0
SYSCALL getegid, SYS_GETEGID, 0
SYSCALL __sys_pipe, SYS_PIPE, 1
SYSCALL __sys_dup, SYS_DUP, 1
SYSCALL __sys_dup2, SYS_DUP2, 2
SYSCALL __sys_brk, SYS_BRK, 1
SYSCALL __sys_unlink, SYS_UNLINK, 1
SYSCALL __sys_chmod, SYS_CHMOD, 2
SYSCALL __sys_mkdir, SYS_MKDIR, 2
SYSCALL __sys_rmdir, SYS_RMDIR, 1
SYSCALL __sys_umask, SYS_UMASK, 1
SYSCALL __sys_chdir, SYS_CHDIR, 1
SYSCALL __sys_chown, SYS_CHOWN, 3
SYSCALL __sys_getcwd, SYS_GETCWD, 2
SYSCALL __sys_getdents, SYS_GETDENTS, 3
SYSCALL __sys_sem_create, POLYOS_SYS_SEM_CREATE, 1
SYSCALL __sys_sem_wait, POLYOS_SYS_SEM_WAIT, 1
SYSCALL __sys_sem_signal, POLYOS_SYS_SEM_SIGNAL, 1
SYSCALL __sys_sem_close, POLYOS_SYS_SEM_CLOSE, 1
SYSCALL kernel_selftest, POLYOS_SYS_KERNEL_SELFTEST, 0

global _exit:function
_exit:
    jmp exit

global __polyos_signal_trampoline:function
__polyos_signal_trampoline:
    mov ebx, [esp+4]
    mov eax, SYS_SIGRETURN
    int 0x80
    ud2

; Generic raw syscall entry for libc modules; errno translation belongs in C.
global __polyos_syscall6:function
__polyos_syscall6:
    push ebp
    push ebx
    push esi
    push edi
    mov eax, [esp+20]
    mov ebx, [esp+24]
    mov ecx, [esp+28]
    mov edx, [esp+32]
    mov esi, [esp+36]
    mov edi, [esp+40]
    mov ebp, [esp+44]
    int 0x80
    pop edi
    pop esi
    pop ebx
    pop ebp
    ret
