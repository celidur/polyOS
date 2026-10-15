#include "syscall_internal.h"
#include <polyos.h>
int pipe2(int fds[2], int flags) {
  return call6(331, (long)fds, flags, 0, 0, 0, 0);
}
int socketpair(int domain, int type, int protocol, int sv[2]) {
  long args[] = {domain, type, protocol, (long)sv};
  return call6(102, 8, (long)args, 0, 0, 0, 0);
}
ssize_t writev(int fd, const struct iovec *iov, int count) {
  return call6(146, fd, (long)iov, count, 0, 0, 0);
}
int poll(struct pollfd *fds, unsigned int count, int timeout) {
  return call6(168, (long)fds, count, timeout, 0, 0, 0);
}
int select(int count, fd_set *readfds, fd_set *writefds, fd_set *exceptfds,
           struct timeval *timeout) {
  if (count < 0 || count > FD_SETSIZE ||
      (timeout && (timeout->tv_sec < 0 || timeout->tv_usec < 0 ||
                   timeout->tv_usec >= 1000000))) {
    errno = EINVAL;
    return -1;
  }
  struct pollfd fds[FD_SETSIZE];
  for (int i = 0; i < count; i++) {
    int r = readfds && FD_ISSET(i, readfds),
        w = writefds && FD_ISSET(i, writefds),
        e = exceptfds && FD_ISSET(i, exceptfds);
    fds[i] = (struct pollfd){.fd = (r || w || e) ? i : -1,
                             .events = (r ? POLLIN : 0) | (w ? POLLOUT : 0),
                             .revents = 0};
  }
  long long ms = timeout ? (long long)timeout->tv_sec * 1000 +
                               (timeout->tv_usec + 999) / 1000
                         : -1;
  int result = poll(fds, count, ms > 2147483647 ? 2147483647 : (int)ms);
  if (result < 0)
    return -1;
  for (int i = 0; i < count; i++)
    if (fds[i].revents & POLLNVAL) {
      errno = EBADF;
      return -1;
    }
  if (readfds)
    FD_ZERO(readfds);
  if (writefds)
    FD_ZERO(writefds);
  if (exceptfds)
    FD_ZERO(exceptfds);
  result = 0;
  for (int i = 0; i < count; i++) {
    if (readfds && (fds[i].events & POLLIN) &&
        (fds[i].revents & (POLLIN | POLLHUP | POLLERR))) {
      FD_SET(i, readfds);
      result++;
    }
    if (writefds && (fds[i].events & POLLOUT) &&
        (fds[i].revents & (POLLOUT | POLLERR))) {
      FD_SET(i, writefds);
      result++;
    }
  }
  return result;
}
