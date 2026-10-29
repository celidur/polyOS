#include "syscall_internal.h"
#include <errno.h>
#include <memory.h>
#include <polyos.h>
#include <stdlib.h>
#define ALIGNMENT 16u
#define LARGE (128u * 1024u)
#define NORMAL 0x424c4b31u
#define MAPPED 0x424c4b32u
#define ALIGNED 0x424c4b33u
typedef struct block {
  size_t size;
  struct block *next;
  u32 free;
  u32 tag;
} block;
typedef char header_alignment[(sizeof(block) == 16) ? 1 : -1];
static block *head;
static volatile int heap_lock, break_lock;
static void lock(volatile int *p) {
  while (__sync_lock_test_and_set(p, 1))
    sched_yield();
}
static void unlock(volatile int *p) { __sync_lock_release(p); }
static u32 raw_break(u32 p) { return __polyos_syscall6(45, p, 0, 0, 0, 0, 0); }
int brk(void *address) {
  lock(&break_lock);
  u32 result = raw_break((u32)address);
  unlock(&break_lock);
  if (result == (u32)address)
    return 0;
  errno = ENOMEM;
  return -1;
}
void *sbrk(intptr_t delta) {
  lock(&break_lock);
  u32 old = raw_break(0);
  s64 wanted = (s64)old + delta;
  if (wanted < 0 || wanted > 0xffffffffLL ||
      raw_break((u32)wanted) != (u32)wanted) {
    unlock(&break_lock);
    errno = ENOMEM;
    return (void *)-1;
  }
  unlock(&break_lock);
  return (void *)old;
}
static void split(block *b, size_t size) {
  if (b->size < size + sizeof(block) + ALIGNMENT)
    return;
  block *next = (void *)((char *)(b + 1) + size);
  *next = (block){b->size - size - sizeof(block), b->next, 1, NORMAL};
  b->next = next;
  b->size = size;
}
static void coalesce(void) {
  for (block *b = head; b && b->next;) {
    block *n = b->next;
    if (b->free && n->free && (char *)(b + 1) + b->size == (char *)n) {
      b->size += sizeof(block) + n->size;
      b->next = n->next;
    } else
      b = n;
  }
}
static void release_tail(void) {
  for (;;) {
    block *prev = 0, *tail = head;
    if (!tail)
      return;
    while (tail->next) {
      prev = tail;
      tail = tail->next;
    }
    if (!tail->free)
      return;
    lock(&break_lock);
    u32 end = (u32)(tail + 1) + tail->size;
    if (raw_break(0) != end || raw_break((u32)tail) != (u32)tail) {
      unlock(&break_lock);
      return;
    }
    unlock(&break_lock);
    if (prev)
      prev->next = 0;
    else
      head = 0;
  }
}
void *malloc(size_t size) {
  if (!size)
    return 0;
  if (size > 0x7fffffff - sizeof(block) - ALIGNMENT) {
    errno = ENOMEM;
    return 0;
  }
  size = (size + ALIGNMENT - 1) & ~(ALIGNMENT - 1);
  if (size >= LARGE) {
    size_t len = (size + sizeof(block) + 4095) & ~4095u;
    block *b = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS,
                    -1, 0);
    if (b == MAP_FAILED)
      return 0;
    *b = (block){len - sizeof(block), 0, len, MAPPED};
    return b + 1;
  }
  lock(&heap_lock);
  block *b = head, *tail = 0;
  for (; b; b = b->next) {
    if (b->free && b->size >= size)
      break;
    tail = b;
  }
  if (!b) {
    lock(&break_lock);
    u32 old = raw_break(0), aligned = (old + ALIGNMENT - 1) & ~(ALIGNMENT - 1),
        end = aligned + sizeof(block) + size;
    if (end < old || raw_break(end) != end) {
      unlock(&break_lock);
      unlock(&heap_lock);
      errno = ENOMEM;
      return 0;
    }
    unlock(&break_lock);
    b = (void *)aligned;
    *b = (block){size, 0, 0, NORMAL};
    if (tail)
      tail->next = b;
    else
      head = b;
  }
  split(b, size);
  b->free = 0;
  unlock(&heap_lock);
  return b + 1;
}
void free(void *ptr) {
  if (!ptr)
    return;
  block *b = (block *)ptr - 1;
  if (b->tag == ALIGNED) {
    free(b->next);
    return;
  }
  if (b->tag == MAPPED) {
    munmap(b, b->free);
    return;
  }
  lock(&heap_lock);
  b->free = 1;
  coalesce();
  release_tail();
  unlock(&heap_lock);
}
void *calloc(size_t count, size_t size) {
  if (size && count > (size_t)-1 / size) {
    errno = ENOMEM;
    return 0;
  }
  size_t total = count * size;
  void *p = malloc(total);
  if (p)
    memset(p, 0, total);
  return p;
}
void *realloc(void *ptr, size_t size) {
  if (!ptr)
    return malloc(size);
  if (!size) {
    free(ptr);
    return 0;
  }
  block *b = (block *)ptr - 1;
  size_t old = b->size;
  if (b->tag == NORMAL && size <= old) {
    lock(&heap_lock);
    split(b, (size + 15) & ~15u);
    coalesce();
    release_tail();
    unlock(&heap_lock);
    return ptr;
  }
  void *p = malloc(size);
  if (!p)
    return 0;
  memcpy(p, ptr, old < size ? old : size);
  free(ptr);
  return p;
}
static int allocate_aligned(void **out, size_t alignment, size_t size) {
  if (!out || alignment < sizeof(void *) || (alignment & (alignment - 1)))
    return EINVAL;
  if (alignment <= ALIGNMENT) {
    void *p = malloc(size);
    if (!p && size)
      return ENOMEM;
    *out = p;
    return 0;
  }
  if (size > (size_t)-1 - alignment - sizeof(block))
    return ENOMEM;
  void *raw = malloc(size + alignment + sizeof(block));
  if (!raw)
    return ENOMEM;
  uintptr_t address =
      ((uintptr_t)raw + sizeof(block) + alignment - 1) & ~(alignment - 1);
  block *b = (block *)address - 1;
  *b = (block){size, raw, 0, ALIGNED};
  *out = (void *)address;
  return 0;
}
int posix_memalign(void **out, size_t alignment, size_t size) {
  int saved_errno = errno;
  int result = allocate_aligned(out, alignment, size);
  errno = saved_errno;
  return result;
}
void *aligned_alloc(size_t alignment, size_t size) {
  if (!alignment || size % alignment) {
    errno = EINVAL;
    return 0;
  }
  void *p = 0;
  int r = posix_memalign(&p, alignment, size);
  if (r) {
    errno = r;
    return 0;
  }
  return p;
}
void *memalign(size_t alignment, size_t size) {
  void *p = 0;
  int r = posix_memalign(&p, alignment, size);
  if (r) {
    errno = r;
    return 0;
  }
  return p;
}
void __heap_before_fork(void) {
  lock(&heap_lock);
  lock(&break_lock);
}
void __heap_parent_after_fork(void) {
  unlock(&break_lock);
  unlock(&heap_lock);
}
void __heap_after_fork(void) {
  break_lock = 0;
  heap_lock = 0;
}
