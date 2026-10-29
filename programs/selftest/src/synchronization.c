#include <errno.h>
#include <polyos.h>
#include <pthread.h>
static pthread_mutex_t mutex = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t cond = PTHREAD_COND_INITIALIZER;
static pthread_once_t once = PTHREAD_ONCE_INIT;
static int counter, ready, once_count;
static void initialize(void) { __sync_fetch_and_add(&once_count, 1); }
static void *worker(void *arg) {
  pthread_once(&once, initialize);
  for (int i = 0; i < 100; i++) {
    pthread_mutex_lock(&mutex);
    counter++;
    pthread_mutex_unlock(&mutex);
  }
  pthread_mutex_lock(&mutex);
  ready++;
  pthread_cond_signal(&cond);
  pthread_mutex_unlock(&mutex);
  return 0;
}
void test_synchronization(void (*expect)(const char *, int, int)) {
  pthread_t threads[4];
  for (int i = 0; i < 4; i++)
    pthread_create(&threads[i], 0, worker, 0);
  pthread_mutex_lock(&mutex);
  while (ready < 4)
    pthread_cond_wait(&cond, &mutex);
  pthread_mutex_unlock(&mutex);
  for (int i = 0; i < 4; i++)
    pthread_join(threads[i], 0);
  expect("mutex protects shared counter", counter == 400, 1);
  expect("pthread_once once only", once_count == 1, 1);
  pthread_mutex_lock(&mutex);
  expect("mutex detects self deadlock", pthread_mutex_lock(&mutex) == EDEADLK,
         1);
  struct timespec end;
  clock_gettime(CLOCK_REALTIME, &end);
  end.tv_nsec += 10000000;
  if (end.tv_nsec >= 1000000000) {
    end.tv_sec++;
    end.tv_nsec -= 1000000000;
  }
  expect("condition timed wait",
         pthread_cond_timedwait(&cond, &mutex, &end) == ETIMEDOUT, 1);
  expect("condition wait reacquires mutex", pthread_mutex_unlock(&mutex) == 0,
         1);
  expect("mutex and condition destroy",
         pthread_mutex_destroy(&mutex) == 0 && pthread_cond_destroy(&cond) == 0,
         1);
  int id = semget(0, 2, 0600);
  expect("SysV semaphore set", id > 0, 1);
  if (id < 0)
    return;
  u16 values[] = {2, 0};
  expect("SysV SETALL", semctl(id, 0, SETALL, (long)values) == 0, 1);
  struct sembuf ops[] = {{0, -1, 0}, {1, 1, 0}};
  expect("SysV atomic multi-op",
         semop(id, ops, 2) == 0 && semctl(id, 0, GETVAL) == 1 &&
             semctl(id, 1, GETVAL) == 1,
         1);
  struct sembuf rollback[] = {{0, -1, 0}, {1, -2, IPC_NOWAIT}};
  errno = 0;
  expect("SysV transaction rollback",
         semop(id, rollback, 2) == -1 && errno == EAGAIN &&
             semctl(id, 0, GETVAL) == 1,
         1);
  expect("SysV SETVAL", semctl(id, 1, SETVAL, 0L) == 0, 1);
  int child = fork(), status;
  if (child == 0) {
    struct sembuf wait = {1, -1, 0};
    _exit(semop(id, &wait, 1) == 0 ? 0 : 99);
  }
  struct timespec delay = {0, 10000000};
  nanosleep(&delay, 0);
  ops[0] = (struct sembuf){1, 1, 0};
  semop(id, ops, 1);
  expect("SysV blocking wake across fork",
         waitpid(child, &status, 0) == child && status == 0, 1);
  child = fork();
  if (child == 0) {
    struct sembuf wait = {1, -1, 0};
    int r = semop(id, &wait, 1);
    _exit(r == -1 && errno == EIDRM ? 0 : 98);
  }
  nanosleep(&delay, 0);
  expect("SysV remove", semctl(id, 0, IPC_RMID) == 0, 1);
  expect("SysV removal wakes waiter",
         waitpid(child, &status, 0) == child && status == 0, 1);
  int keyed = semget(0x504f, 1, IPC_CREAT | IPC_EXCL | 0600);
  expect("SysV key lookup", keyed > 0 && semget(0x504f, 1, 0600) == keyed, 1);
  errno = 0;
  expect("SysV exclusive key",
         semget(0x504f, 1, IPC_CREAT | IPC_EXCL | 0600) == -1 &&
             errno == EEXIST,
         1);
  semctl(keyed, 0, IPC_RMID);
  extern long __polyos_syscall6(long, long, long, long, long, long, long);
  u32 word = 1;
  struct timespec timeout = {0, 10000000};
  expect("timed futex expires",
         __polyos_syscall6(240, (long)&word, 128, 1, (long)&timeout, 0, 0) ==
             -ETIMEDOUT,
         1);
}
