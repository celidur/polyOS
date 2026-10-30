#include <polyos.h>

static void protection_handler(int signal) { _exit(signal == SIGSEGV ? 51 : 99); }
void test_faults(void (*expect)(const char *, int, int)) {
  int irq_status = 0, irq_child = fork();
  if (irq_child == 0) {
    __asm__ volatile("int $0x20");
    _exit(99);
  }
  expect("userspace cannot invoke hardware IRQ gates",
         irq_child > 0 && waitpid(irq_child, &irq_status, 0) == irq_child &&
             WIFSIGNALED(irq_status) && WTERMSIG(irq_status) == SIGSEGV, 1);
  int status = 0, child = fork();
  if (child == 0) {
    setuid(1000);
    __asm__ volatile("cli");
    _exit(99);
  }
  expect("user GPF signals child and preserves kernel",
         child > 0 && waitpid(child, &status, 0) == child &&
             WIFSIGNALED(status) && WTERMSIG(status) == SIGSEGV, 1);
  child = fork();
  if (child == 0) {
    signal(SIGSEGV, protection_handler);
    __asm__ volatile("cli");
    _exit(99);
  }
  expect("user GPF reaches installed fault handler",
         child > 0 && waitpid(child, &status, 0) == child &&
             WIFEXITED(status) && WEXITSTATUS(status) == 51, 1);
}
