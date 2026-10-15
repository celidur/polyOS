#include "polyos.h"
#include "syscall_internal.h"
extern int __sys_socketcall(int call, unsigned long *args);
enum {
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
int socket(int domain, int type, int protocol) {
  unsigned long args[] = {
      (unsigned long)domain,
      (unsigned long)type,
      (unsigned long)protocol,
  };
  return syscall_result(__sys_socketcall(SOCKETCALL_SOCKET, args));
}

int sendto(int sockfd, const void *buf, size_t len, int flags,
           const struct sockaddr *dest_addr, socklen_t addrlen) {
  unsigned long args[] = {
      (unsigned long)sockfd, (unsigned long)buf,       (unsigned long)len,
      (unsigned long)flags,  (unsigned long)dest_addr, (unsigned long)addrlen,
  };
  return syscall_result(__sys_socketcall(SOCKETCALL_SENDTO, args));
}

int recvfrom(int sockfd, void *buf, size_t len, int flags,
             struct sockaddr *src_addr, socklen_t *addrlen) {
  unsigned long args[] = {
      (unsigned long)sockfd, (unsigned long)buf,      (unsigned long)len,
      (unsigned long)flags,  (unsigned long)src_addr, (unsigned long)addrlen,
  };
  return syscall_result(__sys_socketcall(SOCKETCALL_RECVFROM, args));
}

int bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen) {
  unsigned long args[] = {
      (unsigned long)sockfd,
      (unsigned long)addr,
      (unsigned long)addrlen,
  };
  return syscall_result(__sys_socketcall(SOCKETCALL_BIND, args));
}

int connect(int sockfd, const struct sockaddr *addr, socklen_t addrlen) {
  unsigned long args[] = {
      (unsigned long)sockfd,
      (unsigned long)addr,
      (unsigned long)addrlen,
  };
  return syscall_result(__sys_socketcall(SOCKETCALL_CONNECT, args));
}

int listen(int sockfd, int backlog) {
  unsigned long args[] = {
      (unsigned long)sockfd,
      (unsigned long)backlog,
  };
  return syscall_result(__sys_socketcall(SOCKETCALL_LISTEN, args));
}

int accept(int sockfd, struct sockaddr *addr, socklen_t *addrlen) {
  unsigned long args[] = {
      (unsigned long)sockfd,
      (unsigned long)addr,
      (unsigned long)addrlen,
  };
  return syscall_result(__sys_socketcall(SOCKETCALL_ACCEPT, args));
}

ssize_t send(int sockfd, const void *buf, size_t len, int flags) {
  unsigned long args[] = {
      (unsigned long)sockfd,
      (unsigned long)buf,
      (unsigned long)len,
      (unsigned long)flags,
  };
  return syscall_result(__sys_socketcall(SOCKETCALL_SEND, args));
}

ssize_t recv(int sockfd, void *buf, size_t len, int flags) {
  unsigned long args[] = {
      (unsigned long)sockfd,
      (unsigned long)buf,
      (unsigned long)len,
      (unsigned long)flags,
  };
  return syscall_result(__sys_socketcall(SOCKETCALL_RECV, args));
}

int getsockname(int sockfd, struct sockaddr *addr, socklen_t *addrlen) {
  unsigned long args[] = {
      (unsigned long)sockfd,
      (unsigned long)addr,
      (unsigned long)addrlen,
  };
  return syscall_result(__sys_socketcall(SOCKETCALL_GETSOCKNAME, args));
}

int getpeername(int sockfd, struct sockaddr *addr, socklen_t *addrlen) {
  unsigned long args[] = {
      (unsigned long)sockfd,
      (unsigned long)addr,
      (unsigned long)addrlen,
  };
  return syscall_result(__sys_socketcall(SOCKETCALL_GETPEERNAME, args));
}

int setsockopt(int sockfd, int level, int optname, const void *optval,
               socklen_t optlen) {
  unsigned long args[] = {
      (unsigned long)sockfd, (unsigned long)level,  (unsigned long)optname,
      (unsigned long)optval, (unsigned long)optlen,
  };
  return syscall_result(__sys_socketcall(SOCKETCALL_SETSOCKOPT, args));
}
