#include "syscall_internal.h"
#include <polyos.h>
#include <stdarg.h>
int semget(key_t key, int count, int flags) {
  return call6(117, 2, key, count, flags, 0, 0);
}
int semop(int id, struct sembuf *ops, size_t count) {
  return call6(117, 1, id, count, 0, (long)ops, 0);
}
int semctl(int id, int number, int command, ...) {
  long value = 0;
  if (command == SETVAL || command == GETALL || command == SETALL) {
    va_list ap;
    va_start(ap, command);
    value = va_arg(ap, long);
    va_end(ap);
  }
  return call6(117, 3, id, number, command, (long)&value, 0);
}
