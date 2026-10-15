#include "errno.h"
#include "memory.h"
#include "polyos.h"
#include "stdio.h"

void test_udp(void (*expect)(const char *, int, int)) {
  int fd = socket(AF_INET, SOCK_DGRAM, 0);
  expect("UDP socket default protocol", fd >= 0, fd);
  if (fd < 0)
    return;
  struct sockaddr_in local = {0};
  local.sin_family = AF_INET;
  expect("UDP bind ephemeral",
         bind(fd, (struct sockaddr *)&local, sizeof(local)) == 0, errno);
  socklen_t size = sizeof(local);
  expect("UDP assigned port",
         getsockname(fd, (struct sockaddr *)&local, &size) == 0 &&
             local.sin_port != 0,
         errno);
  int other = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
  if (other >= 0) {
    expect("UDP bind collision",
           bind(other, (struct sockaddr *)&local, sizeof(local)) == -1 &&
               errno == EADDRINUSE,
           errno);
    close(other);
  }
  struct sockaddr_in peer = {0};
  peer.sin_family = AF_INET;
  peer.sin_addr.s_addr = 0x0a000202;
  peer.sin_port = htons(31337);
  expect("UDP connect",
         connect(fd, (struct sockaddr *)&peer, sizeof(peer)) == 0, errno);
  const char payload[] = "polyos UDP payload";
  int sent = -1;
  struct timespec delay = {0, 10000000};
  for (int i = 0; i < 100; ++i) {
    sent = send(fd, payload, sizeof(payload), 0);
    if (sent >= 0 || errno != EAGAIN)
      break;
    nanosleep(&delay, NULL);
  }
  expect("UDP transmits payload", sent == sizeof(payload), sent);
  char response[64];
  int got = recvfrom_wait(fd, response, sizeof(response), 0, NULL, NULL, 2000);
  expect("UDP receives echo payload",
         got == sizeof(payload) && !memcmp(response, payload, sizeof(payload)),
         got);
  expect("UDP rejects invalid buffer",
         send(fd, (void *)1, 8, 0) == -1 && errno == EFAULT, errno);
  expect("UDP enforces packet size",
         send(fd, response, 1473, 0) == -1 && errno == EMSGSIZE, errno);
  close(fd);
}
