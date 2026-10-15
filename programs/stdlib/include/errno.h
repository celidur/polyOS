#ifndef POLYOS_ERRNO_H
#define POLYOS_ERRNO_H

int *__errno_location(void);
#define errno (*__errno_location())

#define EPERM 1
#define ENOENT 2
#define ESRCH 3
#define EINTR 4
#define EIO 5
#define EBADF 9
#define ECHILD 10
#define EAGAIN 11
#define ENOMEM 12
#define EACCES 13
#define EFAULT 14
#define EEXIST 17
#define ENODEV 19
#define ENOTDIR 20
#define EISDIR 21
#define EINVAL 22
#define EMFILE 24
#define ENOTTY 25
#define EPIPE 32
#define EDEADLK 35
#define ENOSYS 38
#define ENOTEMPTY 39
#define EMSGSIZE 90
#define ENOTSUP 95
#define EADDRINUSE 98
#define ENETDOWN 100
#define ENOTCONN 107

#endif
