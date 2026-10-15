#include "errno.h"
#include "memory.h"
#include "polyos.h"
#include "stdio.h"

static int control(unsigned int request, unsigned long arg) {
  int fd = open("/dev/net0", O_RDWR, 0);
  if (fd < 0)
    return -1;
  int result = ioctl(fd, request, arg);
  int saved = errno;
  close(fd);
  errno = saved;
  return result;
}
int network_info(struct network_info *info) {
  if (!info) {
    errno = EFAULT;
    return -1;
  }
  return control(0x5101, (unsigned long)info);
}
int network_dhcp_discover(void) { return control(0x5102, 0); }
int recvfrom_wait(int fd, void *buf, size_t len, int flags,
                  struct sockaddr *src, socklen_t *addrlen, u32 timeout_ticks) {
  struct timespec start, now, delay = {0, 1000000};
  if (clock_gettime(CLOCK_MONOTONIC, &start) < 0)
    return -1;
  for (;;) {
    int result = recvfrom(fd, buf, len, flags, src, addrlen);
    if (result >= 0 || errno != EAGAIN)
      return result;
    if (timeout_ticks) {
      if (clock_gettime(CLOCK_MONOTONIC, &now) < 0)
        return -1;
      long long elapsed = (long long)(now.tv_sec - start.tv_sec) * 1000000000 +
                          now.tv_nsec - start.tv_nsec;
      if (elapsed >= (long long)timeout_ticks * 1000000) {
        errno = EAGAIN;
        return -1;
      }
    }
    if (nanosleep(&delay, NULL) < 0)
      return -1;
  }
}
int network_ping_ipv4(u32 ip) {
  int fd = socket(AF_INET, SOCK_RAW, IPPROTO_ICMP);
  if (fd < 0)
    return -1;
  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = ip;
  int result = sendto(fd, NULL, 0, 0, (struct sockaddr *)&addr, sizeof(addr));
  int saved = errno;
  close(fd);
  errno = saved;
  return result < 0 ? -1 : 0;
}
int network_ping_gateway(void) {
  struct network_info info;
  if (network_info(&info) < 0)
    return -1;
  if (info.dhcp_state != 3) {
    errno = ENETDOWN;
    return -1;
  }
  return network_ping_ipv4(((u32)info.router[0] << 24) |
                           ((u32)info.router[1] << 16) |
                           ((u32)info.router[2] << 8) | info.router[3]);
}
int network_dns_query(const char *name) {
  u32 address;
  return gethostbyname_ipv4(name, &address);
}
int network_ping_name(const char *name) {
  u32 address;
  if (gethostbyname_ipv4(name, &address) < 0)
    return -1;
  return network_ping_ipv4(address);
}
