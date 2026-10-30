#include <errno.h>
#include <polyos.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static char text[16384];
static int contains(const char *s, const char *needle) {
  int len = strlen(needle);
  for (; *s; s++)
    if (!strncmp(s, needle, len))
      return 1;
  return 0;
}
static void pathname(char *out, const char *pid, const char *leaf) {
  strcpy(out, "/proc/");
  strcpy(out + 6, pid);
  out[6 + strlen(pid)] = '/';
  strcpy(out + 7 + strlen(pid), leaf);
}
static int load(const char *path) {
  int fd = open(path, O_RDONLY, 0);
  if (fd < 0)
    return -1;
  int n = read(fd, text, sizeof(text) - 1);
  close(fd);
  if (n >= 0)
    text[n] = 0;
  return n;
}
void test_proc_views(void (*expect)(const char *, int, int)) {
  char path[96], pid[24];
  strcpy(pid, itoa(getpid()));
  pathname(path, pid, "status");
  expect("numeric PID status view",
         load(path) > 0 && contains(text, "Threads:\t") &&
             contains(text, "Uid:\t"),
         1);
  pathname(path, pid, "cmdline");
  int n = load(path);
  expect("proc cmdline retains NUL argument separators",
         n >= 18 && !strncmp(text, "/bin/selftest.elf", 17) && text[17] == 0,
         n);
  pathname(path, pid, "stat");
  expect("proc stat uses numeric PID prefix",
         load(path) > 0 && !strncmp(text, pid, strlen(pid)) &&
             contains(text, ") "),
         1);
  DIR *dir = opendir("/proc");
  int found = 0;
  struct dirent *entry;
  if (dir) {
    while ((entry = readdir(dir)))
      if (!strncmp(entry->d_name, pid, strlen(pid) + 1))
        found = 1;
    closedir(dir);
  }
  expect("proc root enumerates live PIDs", found, 1);

  void *page =
      mmap(0, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  int snapshot = open("/proc/self/maps", O_RDONLY, 0);
  mprotect(page, 4096, PROT_NONE);
  n = read(snapshot, text, sizeof(text) - 1);
  if (n >= 0)
    text[n] = 0;
  expect("proc maps open retains snapshot",
         n > 0 && contains(text, "rw-p") && contains(text, "[anon]"), 1);
  close(snapshot);
  expect("proc maps reflects current protection",
         load("/proc/self/maps") > 0 && contains(text, "---p") &&
             contains(text, "[stack]"),
         1);
  munmap(page, 4096);
  expect("proc syscall metrics readable",
         load("/proc/metrics") > 0 && contains(text, "syscall calls"), 1);
  expect("proc timing identifies cycle units",
         load("/proc/timing") > 0 && contains(text, "total_cycles max_cycles"),
         1);
  int child = fork(), status = -1;
  if (!child) {
    setuid(1000);
    errno = 0;
    int fd = open(path, O_RDONLY, 0); /* public stat is readable */
    if (fd >= 0)
      close(fd);
    pathname(path, pid, "maps");
    fd = open(path, O_RDONLY, 0);
    _exit(fd == -1 && errno == EACCES ? 0 : 97);
  }
  expect("proc sensitive maps reject another UID",
         child > 0 && waitpid(child, &status, 0) == child && status == 0, 1);
  errno = 0;
  expect("proc rejects nonexistent PID",
         open("/proc/4294967295/status", 0, 0) == -1 && errno == ENOENT, 1);
}
