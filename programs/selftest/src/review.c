#include <errno.h>
#include <memory.h>
#include <polyos.h>
#include <pthread.h>
#include <stdio.h>

static pthread_once_t once = PTHREAD_ONCE_INIT;
static volatile int initializing, release_initializer, child_initialized;
static void slow_init(void) {
  initializing = 1;
  while (!release_initializer)
    sched_yield();
}
static void child_init(void) { child_initialized = 1; }
static void *once_worker(void *unused) {
  pthread_once(&once, slow_init);
  return 0;
}
static int bounded_wait(pid_t child, int *status) {
  struct timespec delay = {0, 1000000};
  for (int i = 0; i < 200; i++) {
    int result = waitpid(child, status, WNOHANG);
    if (result != 0)
      return result;
    nanosleep(&delay, 0);
  }
  kill(child, SIGKILL);
  waitpid(child, status, 0);
  return -1;
}
void test_review(void (*expect)(const char *, int, int)) {
  char *page =
      mmap(0, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  if (page == MAP_FAILED) {
    expect("review mapping", 0, 1);
    return;
  }
  memset(page, 'A', 4096);
  int status = 0, child = fork();
  if (child == 0)
    _exit(getcwd(page, 128) ? 0 : 90);
  expect("kernel usercopy preserves fork isolation",
         waitpid(child, &status, 0) == child && status == 0 && page[0] == 'A',
         1);
  munmap(page, 4096);

  pthread_t worker;
  pthread_create(&worker, 0, once_worker, 0);
  while (!initializing)
    sched_yield();
  child = fork();
  if (child == 0) {
    pthread_once(&once, child_init);
    _exit(child_initialized ? 0 : 89);
  }
  expect("fork child can restart abandoned pthread_once",
         bounded_wait(child, &status) == child && status == 0, 1);
  release_initializer = 1;
  pthread_join(worker, 0);

  child = fork();
  if (child == 0) {
    for (;;)
      pause();
  }
  kill(child, SIGCONT);
  expect("CONT on running child produces no wait event",
         waitpid(child, &status, WCONTINUED | WNOHANG) == 0, 1);
  kill(child, SIGKILL);
  waitpid(child, &status, 0);

  char *stack = mmap(0, 8192, 3, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  stack_t ss = {stack, 0, 8192}, query;
  sigaltstack(&ss, &ss);
  sigaltstack(0, &query);
  expect("sigaltstack input and output may alias",
         query.ss_sp == stack && query.ss_flags == 0, 1);
  ss.ss_flags = SS_DISABLE;
  sigaltstack(&ss, 0);
  munmap(stack, 8192);

  mkdir("/tmp/review-private", 0700);
  int fd = open("/tmp/review-private/file", O_CREAT | O_RDWR, 0644);
  close(fd);
  child = fork();
  if (child == 0) {
    setuid(1000);
    errno = 0;
    fd = open("/tmp/review-private/file", O_RDONLY, 0);
    _exit(fd == -1 && errno == EACCES ? 0 : 88);
  }
  expect("lookup checks parent directory search permission",
         waitpid(child, &status, 0) == child && status == 0, 1);
  unlink("/tmp/review-private/file");
  rmdir("/tmp/review-private");
  mkdir("/tmp/review-readonly", 0555);
  child = fork();
  if (child == 0) {
    setuid(1000);
    errno = 0;
    fd = open("/tmp/review-readonly/forbidden", O_CREAT | O_RDWR, 0600);
    _exit(fd == -1 && errno == EACCES ? 0 : 87);
  }
  expect("create checks parent directory write permission",
         waitpid(child, &status, 0) == child && status == 0, 1);
  unlink("/tmp/review-readonly/forbidden");
  rmdir("/tmp/review-readonly");

  char *argv[] = {"selftest.elf", "--thread-exec-child", 0};
  char *env[] = {"PATH=/missing-path", 0};
  pid_t spawned = -1;
  int error = posix_spawnp(&spawned, "selftest.elf", 0, 0, argv, env);
  expect("spawnp searches parent PATH with separate child environment",
         error == 0 && waitpid(spawned, &status, 0) == spawned &&
             WEXITSTATUS(status) == 43,
         1);
}
