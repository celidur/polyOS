#ifndef POLYOS_SYSCALL_INTERNAL_H
#define POLYOS_SYSCALL_INTERNAL_H
#include "errno.h"
long __polyos_syscall6(long number, long a, long b, long c, long d, long e, long f);
static inline long syscall_result(long value) {
    if (value < 0 && value >= -4095) {
        errno = -value;
        return -1;
    }
    return value;
}
static inline long call6(long n, long a, long b, long c, long d, long e, long f) {
    return syscall_result(__polyos_syscall6(n, a, b, c, d, e, f));
}
#endif
