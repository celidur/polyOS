#include <errno.h>
#include <polyos.h>
#include <stdlib.h>
void test_allocation(void (*expect)(const char *, int, int)) {
  unsigned char *p = malloc(19);
  expect("malloc 16-byte alignment", p && !((uintptr_t)p & 15), 1);
  if (!p)
    return;
  for (int i = 0; i < 19; i++)
    p[i] = i;
  unsigned char *q = realloc(p, 8192);
  expect("realloc grows preserving content", q && q[18] == 18, 1);
  if (!q) {
    free(p);
    return;
  }
  p = q;
  q = realloc(p, 7);
  expect("realloc shrinks preserving content", q && q[6] == 6, 1);
  p = q;
  errno = 0;
  q = realloc(p, (size_t)-1);
  expect("realloc failure keeps allocation", !q && errno == ENOMEM && p[6] == 6,
         1);
  free(p);
  p = calloc(33, 7);
  int zero = p != 0;
  for (int i = 0; p && i < 231; i++)
    if (p[i])
      zero = 0;
  expect("calloc zeroes", zero, 1);
  free(p);
  errno = 0;
  expect("calloc overflow", calloc((size_t)-1, 2) == 0 && errno == ENOMEM, 1);
  void *aligned = (void *)123;
  expect("posix_memalign invalid leaves output",
         posix_memalign(&aligned, 3, 20) == EINVAL && aligned == (void *)123,
         1);
  expect("posix_memalign page alignment",
         posix_memalign(&aligned, 4096, 123) == 0 &&
             !((uintptr_t)aligned & 4095),
         1);
  if (aligned != (void *)123)
    free(aligned);
  errno = 0;
  expect("aligned_alloc size constraint",
         aligned_alloc(64, 65) == 0 && errno == EINVAL, 1);
  p = aligned_alloc(64, 128);
  expect("aligned_alloc aligned", p && !((uintptr_t)p & 63), 1);
  free(p);
  p = malloc(2 * 1024 * 1024);
  expect("large malloc mapping", p != 0, 1);
  if (p) {
    p[0] = 77;
    p[2 * 1024 * 1024 - 1] = 88;
    int child = fork(), status;
    if (child == 0) {
      p[0] = 99;
      _exit(p[2 * 1024 * 1024 - 1] == 88 ? 0 : 1);
    }
    expect("large allocation fork isolation",
           waitpid(child, &status, 0) == child && status == 0 && p[0] == 77, 1);
    free(p);
  }
  void *old = sbrk(0);
  expect("sbrk observes direct brk",
         brk((char *)old + 4096) == 0 && sbrk(0) == (char *)old + 4096, 1);
  brk(old);
}
