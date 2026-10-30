#include <errno.h>
#include <polyos.h>
#include <pthread.h>
#include <string.h>
static int delayed_fd;
static void *delayed(void *unused) {
  struct timespec t = {0, 20000000};
  nanosleep(&t, 0);
  write(delayed_fd, "Q", 1);
  return 0;
}
void test_ipc(void (*expect)(const char *, int, int)) {
  int packets[2];
  char packet[8];
  int made = socketpair(AF_UNIX, SOCK_SEQPACKET | SOCK_NONBLOCK, 0, packets);
  expect("seqpacket socketpair", made == 0, 1);
  if (made == 0) {
    expect("seqpacket atomic records",
           write(packets[0], "abc", 3) == 3 && write(packets[0], "XY", 2) == 2,
           1);
    expect("seqpacket truncates one record",
           read(packets[1], packet, 2) == 2 && strncmp(packet, "ab", 2) == 0,
           1);
    expect("seqpacket preserves next record",
           read(packets[1], packet, 8) == 2 && strncmp(packet, "XY", 2) == 0,
           1);
    expect("seqpacket empty record", write(packets[0], "", 0) == 0, 1);
    struct pollfd ready = {packets[1], POLLIN, 0};
    expect("seqpacket empty record readiness",
           poll(&ready, 1, 0) == 1 && (ready.revents & POLLIN) &&
               read(packets[1], packet, 8) == 0,
           1);
    errno = 0;
    expect("seqpacket empty queue",
           read(packets[1], packet, 8) == -1 && errno == EAGAIN, 1);
    static char oversized[4097];
    errno = 0;
    expect("seqpacket rejects oversized record",
           write(packets[0], oversized, sizeof(oversized)) == -1 &&
               errno == EMSGSIZE,
           1);
    close(packets[0]);
    expect("seqpacket peer EOF", read(packets[1], packet, 8) == 0, 1);
    close(packets[1]);
  }
  int fds[2], sv[2];
  char buf[16];
  expect("pipe2 flags", pipe2(fds, O_NONBLOCK | O_CLOEXEC) == 0, 1);
  expect("pipe2 cloexec", fcntl(fds[0], F_GETFD, 0) == FD_CLOEXEC, 1);
  errno = 0;
  expect("pipe2 empty nonblock", read(fds[0], buf, 1) == -1 && errno == EAGAIN,
         1);
  int alias = dup(fds[0]);
  fcntl(alias, F_SETFL, 0);
  expect("dup shares status flags", !(fcntl(fds[0], F_GETFL, 0) & O_NONBLOCK),
         1);
  close(alias);
  struct iovec vecs[] = {{"ab", 2}, {"cde", 3}};
  expect("writev gathers", writev(fds[1], vecs, 2) == 5, 1);
  expect("writev payload",
         read(fds[0], buf, 5) == 5 && strncmp(buf, "abcde", 5) == 0, 1);
  errno = 0;
  expect("writev rejects count",
         writev(fds[1], vecs, -1) == -1 && errno == EINVAL, 1);
  struct pollfd p = {fds[0], POLLIN, 0};
  expect("poll empty", poll(&p, 1, 0) == 0, 1);
  delayed_fd = fds[1];
  pthread_t t;
  pthread_create(&t, 0, delayed, 0);
  expect("poll blocks until data",
         poll(&p, 1, 1000) == 1 && (p.revents & POLLIN), 1);
  pthread_join(t, 0);
  read(fds[0], buf, 1);
  struct timespec before, after;
  clock_gettime(CLOCK_MONOTONIC, &before);
  expect("poll timeout", poll(&p, 1, 20) == 0, 1);
  clock_gettime(CLOCK_MONOTONIC, &after);
  expect("poll elapsed",
         (after.tv_sec - before.tv_sec) * 1000000000LL + after.tv_nsec -
                 before.tv_nsec >=
             20000000,
         1);
  close(fds[1]);
  expect("poll EOF", poll(&p, 1, 0) == 1 && (p.revents & POLLHUP), 1);
  close(fds[0]);
  p = (struct pollfd){127, POLLIN, 0};
  expect("poll invalid fd", poll(&p, 1, 0) == 1 && p.revents == POLLNVAL, 1);
  expect("socketpair", socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0, 1);
  expect("socketpair send", send(sv[0], "hello", 5, 0) == 5, 1);
  expect("socketpair recv",
         recv(sv[1], buf, 5, 0) == 5 && strncmp(buf, "hello", 5) == 0, 1);
  expect("socketpair reverse",
         write(sv[1], "R", 1) == 1 && read(sv[0], buf, 1) == 1 && buf[0] == 'R',
         1);
  fd_set reads;
  FD_ZERO(&reads);
  FD_SET(sv[0], &reads);
  struct timeval zero = {0, 0};
  expect("select empty", select(sv[0] + 1, &reads, 0, 0, &zero) == 0, 1);
  write(sv[1], "S", 1);
  FD_SET(sv[0], &reads);
  expect("select readable",
         select(sv[0] + 1, &reads, 0, 0, &zero) == 1 && FD_ISSET(sv[0], &reads),
         1);
  read(sv[0], buf, 1);
  int child = fork();
  if (child == 0) {
    close(sv[0]);
    write(sv[1], "F", 1);
    close(sv[1]);
    _exit(0);
  }
  close(sv[1]);
  expect("socketpair fork", read(sv[0], buf, 1) == 1 && buf[0] == 'F', 1);
  expect("socketpair peer EOF", read(sv[0], buf, 1) == 0, 1);
  int status;
  waitpid(child, &status, 0);
  close(sv[0]);

  pipe2(fds, O_NONBLOCK);
  char capacity[4096];
  for (int i = 0; i < 4096; i++)
    capacity[i] = 'A';
  expect("pipe capacity", write(fds[1], capacity, 4095) == 4095, 1);
  errno = 0;
  expect("pipe atomic small write",
         write(fds[1], "XY", 2) == -1 && errno == EAGAIN, 1);
  expect("pipe remaining byte", write(fds[1], "Z", 1) == 1, 1);
  errno = 0;
  expect("pipe full", write(fds[1], "Z", 1) == -1 && errno == EAGAIN, 1);
  expect("pipe atomic contents",
         read(fds[0], capacity, 4096) == 4096 && capacity[4095] == 'Z', 1);
  close(fds[0]);
  signal(SIGPIPE, SIG_IGN);
  errno = 0;
  expect("pipe broken error", write(fds[1], "Z", 1) == -1 && errno == EPIPE, 1);
  signal(SIGPIPE, SIG_DFL);
  close(fds[1]);
  child = fork();
  if (child == 0) {
    pipe(fds);
    close(fds[0]);
    write(fds[1], "X", 1);
    _exit(99);
  }
  waitpid(child, &status, 0);
  expect("pipe broken signal",
         WIFSIGNALED(status) && WTERMSIG(status) == SIGPIPE, 1);
}
