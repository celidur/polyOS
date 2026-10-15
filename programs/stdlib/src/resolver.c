#include "errno.h"
#include "memory.h"
#include "polyos.h"
#include "string.h"

#define DNS_PORT 53
#define DNS_TYPE_A 1
#define DNS_CLASS_IN 1
#define DNS_FLAG_RESPONSE 0x8000
#define DNS_FLAG_RD 0x0100

static void dns_write_be16(u8 *buf, int offset, u16 value) {
  buf[offset] = value >> 8;
  buf[offset + 1] = value & 0xff;
}

static u16 dns_read_be16(const u8 *buf, int offset) {
  return ((u16)buf[offset] << 8) | buf[offset + 1];
}

static int dns_build_query(const char *name, u16 id, u8 *buf, int max_len) {
  int offset = 12;
  int label_start = 0;
  int name_len = strlen(name);

  if (name_len <= 0 || name_len > 253 || max_len < 18) {
    errno = EINVAL;
    return -1;
  }

  memset(buf, 0, max_len);
  dns_write_be16(buf, 0, id);
  dns_write_be16(buf, 2, DNS_FLAG_RD);
  dns_write_be16(buf, 4, 1);

  for (int i = 0; i <= name_len; i++) {
    if (name[i] != '.' && name[i] != '\0') {
      continue;
    }

    int label_len = i - label_start;
    if (label_len <= 0 || label_len > 63 ||
        offset + label_len + 1 + 5 > max_len) {
      errno = EINVAL;
      return -1;
    }

    buf[offset++] = label_len;
    for (int j = 0; j < label_len; j++) {
      buf[offset++] = name[label_start + j];
    }
    label_start = i + 1;
  }

  buf[offset++] = 0;
  dns_write_be16(buf, offset, DNS_TYPE_A);
  offset += 2;
  dns_write_be16(buf, offset, DNS_CLASS_IN);
  offset += 2;

  return offset;
}

static int dns_skip_name(const u8 *packet, int packet_len, int offset) {
  while (offset < packet_len) {
    u8 len = packet[offset];
    if ((len & 0xc0) == 0xc0) {
      return offset + 2 <= packet_len ? offset + 2 : -1;
    }
    if ((len & 0xc0) != 0) {
      return -1;
    }
    if (len == 0) {
      return offset + 1;
    }
    offset += 1 + len;
  }

  return -1;
}

static int dns_parse_response(const u8 *packet, int packet_len, u16 id,
                              u32 *out_addr) {
  if (packet_len < 12 || dns_read_be16(packet, 0) != id) {
    errno = EINVAL;
    return -1;
  }

  u16 flags = dns_read_be16(packet, 2);
  if ((flags & DNS_FLAG_RESPONSE) == 0 || (flags & 0x000f) != 0) {
    errno = ENOENT;
    return -1;
  }

  int question_count = dns_read_be16(packet, 4);
  int answer_count = dns_read_be16(packet, 6);
  int offset = 12;

  for (int i = 0; i < question_count; i++) {
    offset = dns_skip_name(packet, packet_len, offset);
    if (offset < 0 || offset + 4 > packet_len) {
      errno = EINVAL;
      return -1;
    }
    offset += 4;
  }

  for (int i = 0; i < answer_count; i++) {
    offset = dns_skip_name(packet, packet_len, offset);
    if (offset < 0 || offset + 10 > packet_len) {
      errno = EINVAL;
      return -1;
    }

    u16 record_type = dns_read_be16(packet, offset);
    u16 record_class = dns_read_be16(packet, offset + 2);
    u16 record_len = dns_read_be16(packet, offset + 8);
    offset += 10;

    if (offset + record_len > packet_len) {
      errno = EINVAL;
      return -1;
    }

    if (record_type == DNS_TYPE_A && record_class == DNS_CLASS_IN &&
        record_len == 4) {
      *out_addr = ((u32)packet[offset] << 24) |
                  ((u32)packet[offset + 1] << 16) |
                  ((u32)packet[offset + 2] << 8) | packet[offset + 3];
      return 0;
    }

    offset += record_len;
  }

  errno = ENOENT;
  return -1;
}

static void resolver_sleep_ms(int milliseconds) {
  struct timespec req;
  req.tv_sec = milliseconds / 1000;
  req.tv_nsec = (milliseconds % 1000) * 1000000;
  nanosleep(&req, NULL);
}

int gethostbyname_ipv4(const char *name, u32 *out_addr) {
  if (!name || !out_addr) {
    errno = EFAULT;
    return -1;
  }

  struct network_info info;
  if (network_info(&info) < 0 || !info.present || info.dhcp_state != 3) {
    errno = ENETDOWN;
    return -1;
  }

  int fd = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
  if (fd < 0) {
    return -1;
  }

  u8 query[256];
  static unsigned int sequence;
  u16 id = (u16)__sync_add_and_fetch(&sequence, 1);
  int query_len = dns_build_query(name, id, query, sizeof(query));
  if (query_len < 0) {
    close(fd);
    return -1;
  }

  u32 dns_ip = ((u32)info.dns[0] << 24) | ((u32)info.dns[1] << 16) |
               ((u32)info.dns[2] << 8) | info.dns[3];
  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_port = htons(DNS_PORT);
  addr.sin_addr.s_addr = dns_ip;

  if (connect(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
    int saved = errno;
    close(fd);
    errno = saved;
    return -1;
  }
  int sent = -1;
  for (int i = 0; i < 20; i++) {
    sent =
        sendto(fd, query, query_len, 0, (struct sockaddr *)&addr, sizeof(addr));
    if (sent == query_len) {
      break;
    }
    if (errno != EAGAIN) {
      close(fd);
      return -1;
    }
    resolver_sleep_ms(10);
  }
  if (sent != query_len) {
    close(fd);
    errno = EAGAIN;
    return -1;
  }

  u8 response[512];
  struct sockaddr_in src;
  int received = -1;
  for (int i = 0; i < 200; i++) {
    socklen_t src_len = sizeof(src);
    received = recvfrom(fd, response, sizeof(response), 0,
                        (struct sockaddr *)&src, &src_len);
    if (received >= 0) {
      break;
    }
    if (errno != EAGAIN) {
      close(fd);
      return -1;
    }
    resolver_sleep_ms(10);
  }
  close(fd);
  if (received < 0) {
    errno = EAGAIN;
    return -1;
  }

  return dns_parse_response(response, received, id, out_addr);
}
