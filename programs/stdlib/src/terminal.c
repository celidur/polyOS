#include <errno.h>
#include <polyos.h>
int tcgetattr(int fd, struct termios *s) { return ioctl(fd, TCGETS, (long)s); }
int tcsetattr(int fd, int action, const struct termios *s) {
  if (action < 0 || action > 2) {
    errno = EINVAL;
    return -1;
  }
  return ioctl(fd, TCSETS + action, (long)s);
}
pid_t tcgetpgrp(int fd) {
  pid_t group;
  if (ioctl(fd, TIOCGPGRP, (long)&group) < 0)
    return -1;
  return group;
}
int tcsetpgrp(int fd, pid_t group) {
  return ioctl(fd, TIOCSPGRP, (long)&group);
}
int tcflush(int fd, int queue) { return ioctl(fd, TCFLSH, queue); }
int isatty(int fd) {
  struct termios settings;
  return tcgetattr(fd, &settings) == 0;
}
void cfmakeraw(struct termios *s) {
  s->c_iflag = 0;
  s->c_oflag = 0;
  s->c_lflag = 0;
  s->c_cflag = CS8 | CREAD | CLOCAL;
  s->c_cc[VMIN] = 1;
  s->c_cc[VTIME] = 0;
}
