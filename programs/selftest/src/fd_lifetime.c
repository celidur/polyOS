#include <errno.h>
#include <polyos.h>
#include <stdio.h>

void test_fd_lifetime(void (*expect)(const char *, int, int)) {
  int pipes[2], pair[2], fds[128], count = 0;
  pipe(pipes);
  socketpair(AF_UNIX, SOCK_STREAM, 0, pair);
  while (count < 128) {
    int fd = open("/dev/null", O_RDONLY, 0);
    if (fd < 0) break;
    fds[count++] = fd;
  }
  errno = 0;
  expect("failed dup reports descriptor exhaustion", dup(pipes[1]) == -1 && errno == EMFILE, 1);
  errno = 0;
  expect("failed socket dup reports descriptor exhaustion", dup(pair[1]) == -1 && errno == EMFILE, 1);
  close(pipes[1]);
  close(pair[1]);
  struct pollfd ready = {pipes[0], POLLIN, 0};
  expect("failed dup preserves pipe EOF", poll(&ready, 1, 0) == 1 && (ready.revents & POLLHUP), 1);
  char byte;
  fcntl(pair[0], F_SETFL, O_NONBLOCK);
  expect("failed dup preserves socketpair EOF", read(pair[0], &byte, 1) == 0, 1);
  for (int i = 0; i < count; i++) close(fds[i]);
  close(pipes[0]); close(pair[0]);

  int fd = open("/dev/null", O_RDONLY, 0);
  dup2(fd, 127);
  close(fd);
  int child = fork(), status = -1, complete = 0;
  if (!child) {
    char *args[] = {"selftest.elf", "--thread-exec-child", 0};
    pid_t spawned;
    int error = posix_spawn(&spawned, "/bin/selftest.elf", 0, 0, args, 0);
    if (!error) waitpid(spawned, &status, 0);
    _exit(!error && WIFEXITED(status) && WEXITSTATUS(status) == 43 ? 0 : 92);
  }
  struct timespec delay = {0, 1000000};
  for (int i = 0; child > 0 && i < 500; i++) {
    if (waitpid(child, &status, WNOHANG) == child) { complete = 1; break; }
    nanosleep(&delay, 0);
  }
  if (child > 0 && !complete) { kill(child, SIGKILL); waitpid(child, &status, 0); }
  expect("spawn completes when highest descriptor is occupied", complete && status == 0, 1);
  close(127);
}
