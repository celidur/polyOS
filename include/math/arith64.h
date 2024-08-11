#ifndef ARITH64_H
#define ARITH64_H

#include <os/types.h>

int64_t __absvdi2(int64_t a);
int64_t __ashldi3(int64_t a, int b);
int64_t __ashrdi3(int64_t a, int b);
int __clzdi2(uint64_t a);
int __clzsi2(uint32_t a);
int __ctzdi2(uint64_t a);
int __ctzsi2(uint32_t a);
int64_t __divdi3(int64_t a, int64_t b);
int __ffsdi2(uint64_t a);
uint64_t __lshrdi3(uint64_t a, int b);
int64_t __moddi3(int64_t a, int64_t b);
int __popcountdi2(uint64_t);
int __popcountsi2(uint32_t a);
uint64_t __udivdi3(uint64_t a, uint64_t b);
uint64_t __umoddi3(uint64_t a, uint64_t b);

#endif // ARITH64_H
