#include <errno.h>
#include <polyos.h>
#include <pthread.h>
static struct pollfd *invalidated;
static void *unmap_poll_array(void *unused) {
  struct timespec delay = {0, 20000000};
  nanosleep(&delay, 0);
  munmap(invalidated, 4096);
  return 0;
}
static int millis(const struct timespec *t) {
  return t->tv_sec * 1000 + t->tv_nsec / 1000000;
}
void test_poll_lifetime(void (*expect)(const char *, int, int)) {
  invalidated = mmap(0, 4096, 3, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  if (invalidated == MAP_FAILED) { expect("poll lifetime mapping", 0, 1); return; }
  *invalidated = (struct pollfd){-1, POLLIN, 0};
  pthread_t thread;
  if (pthread_create(&thread, 0, unmap_poll_array, 0)) {
    munmap(invalidated, 4096); expect("poll lifetime worker", 0, 1); return;
  }
  errno = 0;
  int result = poll(invalidated, 1, 400), error = errno;
  struct timespec start, end;
  clock_gettime(CLOCK_MONOTONIC, &start);
  int next = poll(0, 0, 30);
  clock_gettime(CLOCK_MONOTONIC, &end);
  pthread_join(thread, 0);
  expect("poll detects array unmapped during wait", result == -1 && error == EFAULT, 1);
  int elapsed = millis(&end) - millis(&start);
  expect("poll error clears deadline for next call", next == 0 && elapsed >= 30 && elapsed < 150, elapsed);
}
