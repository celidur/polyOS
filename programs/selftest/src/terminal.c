#include <errno.h>
#include <polyos.h>
#include <stdio.h>
static volatile int interrupt_seen, resize_seen;
static void interrupted(int signal) { interrupt_seen++; }
static void resized(int signal) { resize_seen++; }
void test_terminal(void (*expect)(const char *, int, int)) {
  struct termios saved, settings, check;
  expect("terminal termios layout", sizeof(settings) == 36, 1);
  expect("terminal detected", isatty(0) && tcgetattr(0, &saved) == 0, 1);
  int regular = open("/dev/null", 0, 0);
  errno = 0;
  expect("nonterminal rejected", !isatty(regular) && errno == ENOTTY, 1);
  close(regular);
  pid_t original = getpgrp(), foreground = tcgetpgrp(0);
  setpgid(0, getpid());
  sighandler_t ttou = signal(SIGTTOU, SIG_IGN);
  expect("terminal foreground group",
         tcsetpgrp(0, getpgrp()) == 0 && tcgetpgrp(0) == getpgrp(), 1);
  settings = saved;
  settings.c_iflag = ICRNL;
  settings.c_lflag = ISIG | ICANON | ECHO | ECHOE | ECHOK;
  settings.c_cc[VTIME] = 0;
  settings.c_cc[VMIN] = 1;
  expect("terminal canonical mode", tcsetattr(0, TCSAFLUSH, &settings) == 0, 1);
  int alias = open("/dev/tty", 2, 0);
  expect("terminal settings shared across opens",
         tcgetattr(alias, &check) == 0 && check.c_lflag == settings.c_lflag, 1);
  close(alias);
  printf("tty-canonical-ready\n");
  char buffer[16];
  ssize_t n = read(0, buffer, sizeof(buffer));
  expect("terminal canonical erase and line",
         n == 3 && buffer[0] == 'a' && buffer[1] == 'c' && buffer[2] == '\n',
         1);
  printf("tty-eof-ready\n");
  expect("terminal canonical EOF", read(0, buffer, sizeof(buffer)) == 0, 1);
  cfmakeraw(&settings);
  settings.c_cc[VMIN] = 2;
  expect("terminal raw mode", tcsetattr(0, TCSANOW, &settings) == 0, 1);
  printf("tty-raw-ready\n");
  n = read(0, buffer, 2);
  expect("terminal raw NUL and VMIN",
         n == 2 && buffer[0] == 0 && buffer[1] == 'Z', 1);
  int original_flags = fcntl(0, F_GETFL, 0);
  fcntl(0, F_SETFL, original_flags | O_NONBLOCK);
  printf("tty-nonblock-ready\n");
  struct pollfd ready = {0, POLLIN, 0};
  poll(&ready, 1, 1000);
  expect("terminal nonblock overrides VMIN",
         read(0, buffer, 2) == 1 && buffer[0] == 'N', 1);
  fcntl(0, F_SETFL, original_flags);
  settings.c_cc[VMIN] = 0;
  tcsetattr(0, TCSANOW, &settings);
  expect("terminal VMIN zero immediate", read(0, buffer, 1) == 0, 1);
  settings.c_cc[VTIME] = 1;
  errno = 0;
  expect("terminal rejects unsupported timing",
         tcsetattr(0, TCSANOW, &settings) == -1 && errno == ENOTSUP, 1);
  settings.c_cc[VTIME] = 0;
  settings.c_cc[VMIN] = 1;
  settings.c_lflag = ISIG;
  tcsetattr(0, TCSANOW, &settings);
  signal(SIGINT, interrupted);
  printf("tty-signal-ready\n");
  for (int i = 0; i < 50 && !interrupt_seen; i++) {
    struct timespec delay = {0, 20000000};
    nanosleep(&delay, 0);
  }
  expect("terminal VINTR signals foreground group", interrupt_seen == 1, 1);
  signal(SIGINT, SIG_DFL);
  signal(SIGWINCH, resized);
  struct winsize old, size = {30, 100, 0, 0};
  ioctl(0, TIOCGWINSZ, (long)&old);
  expect("terminal window size and signal",
         ioctl(0, TIOCSWINSZ, (long)&size) == 0 && resize_seen == 1, 1);
  ioctl(0, TIOCSWINSZ, (long)&old);
  signal(SIGWINCH, SIG_DFL);
  int child = fork(), status = 0;
  if (child == 0) {
    setpgid(0, 0);
    char c;
    ssize_t r;
    do {
      r = read(0, &c, 1);
    } while (r < 0 && errno == EINTR);
    _exit(r == 1 && c == 'B' ? 0 : 79);
  }
  expect("terminal background read stops group",
         waitpid(child, &status, WUNTRACED) == child && WIFSTOPPED(status) &&
             WSTOPSIG(status) == SIGTTIN,
         1);
  tcsetpgrp(0, child);
  kill(child, SIGCONT);
  printf("tty-background-ready\n");
  expect("terminal foreground transfer resumes read",
         waitpid(child, &status, 0) == child && WIFEXITED(status) &&
             WEXITSTATUS(status) == 0,
         1);
  tcsetpgrp(0, getpgrp());
  tcsetattr(0, TCSAFLUSH, &saved);
  tcsetpgrp(0, foreground);
  setpgid(0, original);
  signal(SIGTTOU, ttou);
}
