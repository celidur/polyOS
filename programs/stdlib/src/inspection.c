#include "syscall_internal.h"
#include <polyos.h>
extern unsigned long *__polyos_auxv;
int sysinfo(struct sysinfo *info) {
  return call6(116, (long)info, 0, 0, 0, 0, 0);
}
unsigned long getauxval(unsigned long type) {
  if (__polyos_auxv)
    for (unsigned long *p = __polyos_auxv; p[0]; p += 2)
      if (p[0] == type)
        return p[1];
  errno = ENOENT;
  return 0;
}
long sysconf(int name) {
  switch (name) {
  case _SC_PAGESIZE:
    return getauxval(AT_PAGESZ);
  case _SC_CLK_TCK:
    return getauxval(AT_CLKTCK);
  case _SC_OPEN_MAX:
    return 128;
  case _SC_NPROCESSORS_CONF:
  case _SC_NPROCESSORS_ONLN:
    return 1;
  case _SC_PHYS_PAGES:
  case _SC_AVPHYS_PAGES: {
    struct sysinfo s;
    if (sysinfo(&s))
      return -1;
    return (name == _SC_PHYS_PAGES ? s.totalram : s.freeram) / 4096;
  }
  default:
    errno = EINVAL;
    return -1;
  }
}
int dl_iterate_phdr(int (*callback)(struct dl_phdr_info *, size_t, void *),
                    void *data) {
  if (!callback) {
    errno = EINVAL;
    return -1;
  }
  struct dl_phdr_info info = {0, (const char *)getauxval(AT_EXECFN),
                              (const Elf32_Phdr *)getauxval(AT_PHDR),
                              (u16)getauxval(AT_PHNUM)};
  return callback(&info, sizeof(info), data);
}
