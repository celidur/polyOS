#include <errno.h>
#include <polyos.h>
#include <string.h>
static int headers(struct dl_phdr_info *i, size_t size, void *data) {
  return size == sizeof(*i) && i->dlpi_phnum > 0 && i->dlpi_phdr[0].p_memsz > 0
             ? 73
             : 0;
}
void test_inspection(void (*expect)(const char *, int, int)) {
  struct sysinfo s;
  expect("sysinfo layout", sizeof(s) == 64, 1);
  expect("sysinfo snapshot",
         sysinfo(&s) == 0 && s.totalram > s.freeram && s.freeram > 0 &&
             s.procs >= 2 && s.mem_unit == 1,
         1);
  errno = 0;
  expect("sysinfo invalid pointer", sysinfo((void *)1) == -1 && errno == EFAULT,
         1);
  expect("sysconf page size", sysconf(_SC_PAGESIZE) == 4096, 1);
  expect("sysconf CPU count", sysconf(_SC_NPROCESSORS_ONLN) == 1, 1);
  expect("auxv entry and headers",
         getauxval(AT_ENTRY) != 0 && getauxval(AT_PHNUM) > 0 &&
             getauxval(AT_PHENT) == 32,
         1);
  expect("program header iteration", dl_iterate_phdr(headers, 0) == 73, 1);
  errno = 0;
  expect("auxv missing", getauxval(9999) == 0 && errno == ENOENT, 1);
  const char *paths[] = {"/proc/uptime", "/proc/meminfo", "/proc/stat",
                         "/proc/self/status"};
  for (int i = 0; i < 4; i++) {
    int fd = open(paths[i], 0, 0);
    char b[256];
    expect("proc inspection readable", fd >= 0 && read(fd, b, sizeof(b)) > 0,
           1);
    close(fd);
  }
}
