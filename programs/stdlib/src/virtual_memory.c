#include "syscall_internal.h"
#include <polyos.h>
void *mmap(void *address, size_t length, int prot, int flags, int fd,
           off_t offset) {
  long args[] = {(long)address, length, prot, flags, fd, offset};
  return (void *)call6(90, (long)args, 0, 0, 0, 0, 0);
}
int munmap(void *address, size_t length) {
  return call6(91, (long)address, length, 0, 0, 0, 0);
}
int mprotect(void *address, size_t length, int prot) {
  return call6(125, (long)address, length, prot, 0, 0, 0);
}
