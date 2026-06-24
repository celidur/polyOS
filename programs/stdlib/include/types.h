/* SPDX-License-Identifier: GPL-2.0 */
#ifndef _TYPES_H_
#define _TYPES_H_

// #include <stdbool.h>

#define ARRAY_SIZE(x) (sizeof(x) / sizeof((x)[0]))

typedef unsigned char		u8;
typedef unsigned short		u16;
typedef unsigned int		u32;
typedef unsigned long long	u64;
typedef signed char		    s8;
typedef short			    s16;
typedef int			        s32;
typedef long long	    	s64;

#ifdef __SIZE_TYPE__
typedef __SIZE_TYPE__ size_t;
#else
typedef u32 size_t;
#endif
typedef s32 ssize_t;
typedef s32 off_t;
typedef s32 pid_t;
typedef s32 intptr_t;
typedef u32 uintptr_t;
typedef s32 time_t;
typedef s32 suseconds_t;
typedef s32 clockid_t;

#define CLOCK_REALTIME 0
#define CLOCK_MONOTONIC 1

struct timespec
{
    time_t tv_sec;
    s32 tv_nsec;
};

struct timeval
{
    time_t tv_sec;
    suseconds_t tv_usec;
};

struct timezone
{
    s32 tz_minuteswest;
    s32 tz_dsttime;
};

#define F_DUPFD 0
#define F_GETFD 1
#define F_SETFD 2
#define F_GETFL 3
#define F_SETFL 4
#define FD_CLOEXEC 1
#define O_NONBLOCK 0x800

#define S_IFMT  0170000
#define S_IFREG 0100000
#define S_IFDIR 0040000
#define S_ISREG(mode) (((mode) & S_IFMT) == S_IFREG)
#define S_ISDIR(mode) (((mode) & S_IFMT) == S_IFDIR)

#define DT_UNKNOWN 0
#define DT_DIR 4
#define DT_REG 8

#ifndef POLYOS_FILE_STRUCTS_DEFINED
#define POLYOS_FILE_STRUCTS_DEFINED

typedef unsigned int FILE_STAT_FLAGS;
enum
{
    FILE_STAT_READ_ONLY = 0b00000001,
};

struct file_stat
{
    int size;
    FILE_STAT_FLAGS flags;
    u32 mode;
    u32 uid;
    u32 gid;
    u32 is_dir;
};

struct dirent
{
    u32 d_ino;
    u32 d_off;
    u16 d_reclen;
    u8 d_type;
    char d_name[256];
};

#endif

/* required for opal-api.h */
typedef u8  uint8_t;
typedef u16 uint16_t;
typedef u32 uint32_t;
typedef u64 uint64_t;
typedef s8  int8_t;
typedef s16 int16_t;
typedef s32 int32_t;
typedef s64 int64_t;

#define min(x,y) ({ \
	typeof(x) _x = (x);	\
	typeof(y) _y = (y);	\
	(void) (&_x == &_y);	\
	_x < _y ? _x : _y; })

#define max(x,y) ({ \
	typeof(x) _x = (x);	\
	typeof(y) _y = (y);	\
	(void) (&_x == &_y);	\
	_x > _y ? _x : _y; })

#define min_t(type, a, b) min(((type) a), ((type) b))
#define max_t(type, a, b) max(((type) a), ((type) b))

/*
 * Some compilers expose bool as a macro to _Bool even without explicitly
 * including stdbool.h. Only define it if it is genuinely absent.
 */
#if !defined(__STDC_VERSION__) || __STDC_VERSION__ < 202311L
#ifndef bool
typedef int bool;
#endif

#ifndef true
#define true 1
#endif

#ifndef false
#define false 0
#endif
#endif

#define NULL ((void *)0)

#endif /* _TYPES_H_ */
