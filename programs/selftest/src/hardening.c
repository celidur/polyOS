#include <errno.h>
#include <memory.h>
#include <polyos.h>
#include <pthread.h>
#include <stdio.h>

struct fp_sample {
  double number;
  u32 vector[4];
  u16 control;
};
extern int review_fpu_operation(const struct fp_sample *, struct fp_sample *,
                                int, int, int);
extern void review_fpu_clobber(void);
extern int review_fpu_control(void);
static void *fp_inherited_control(void *unused) {
  return (void *)(u32)review_fpu_control();
}
static struct fp_sample samples[2] = {
    {1.25, {0x11223344, 0x55667788, 0x99aabbcc, 0xddeeff00}, 0x077f},
    {8.5, {0xa1b2c3d4, 0x12345678, 0x87654321, 0x0f0e0d0c}, 0x0b7f},
};
static void *fp_worker(void *arg) {
  struct fp_sample *sample = arg, out;
  for (int i = 0; i < 32; i++) {
    review_fpu_operation(sample, &out, i & 1 ? 158 : 0, 0, 0);
    if (memcmp(sample, &out, 26))
      return (void *)1;
  }
  return 0;
}
static void fp_signal(int signal) { review_fpu_clobber(); }

/* A minimal static ELF32 image, independent of compiler/linker output. */
static void elf_image(u8 *image) {
  memset(image, 0, 128);
  image[0] = 0x7f;
  image[1] = 'E';
  image[2] = 'L';
  image[3] = 'F';
  image[4] = image[5] = image[6] = 1;
  u16 *half = (u16 *)image;
  u32 *word = (u32 *)image;
  half[8] = 2;
  half[9] = 3;
  word[5] = 1;
  word[6] = 0x400000;
  word[7] = 52;
  half[20] = 52;
  half[21] = 32;
  half[22] = 1;
  word[13] = 1;
  word[14] = 116;
  word[15] = 0x400000;
  word[17] = 12;
  word[18] = 12;
  word[19] = 5;
  word[20] = 1;
  const u8 code[] = {0xb8, 1, 0, 0, 0, 0xbb, 37, 0, 0, 0, 0xcd, 0x80};
  memcpy(image + 116, code, sizeof(code));
}
static int run_image(u8 *image, int valid) {
  int fd = open("/tmp/review-image", O_CREAT | O_TRUNC | O_WRONLY, 0700);
  if (fd < 0)
    return 0;
  int written = write(fd, image, 128);
  close(fd);
  if (written != 128)
    return 0;
  int child = fork(), status = 0;
  if (child == 0) {
    char *args[] = {"review-image", 0};
    execve("/tmp/review-image", args, 0);
    _exit(!valid && errno == ENOEXEC ? 0 : 91);
  }
  return child > 0 && waitpid(child, &status, 0) == child &&
         WIFEXITED(status) && WEXITSTATUS(status) == (valid ? 37 : 0);
}
void test_hardening(void (*expect)(const char *, int, int)) {
  pthread_t threads[2];
  int a = pthread_create(&threads[0], 0, fp_worker, &samples[0]);
  int b = pthread_create(&threads[1], 0, fp_worker, &samples[1]);
  void *first = (void *)1, *second = (void *)1;
  if (!a)
    pthread_join(threads[0], &first);
  if (!b)
    pthread_join(threads[1], &second);
  expect("x87, XMM and control word survive yield and timer preemption",
         !a && !b && !first && !second, 1);
  u16 old_control, custom_control = 0x077f;
  __asm__ volatile("fnstcw %0; fldcw %1"
                   : "=m"(old_control)
                   : "m"(custom_control));
  a = pthread_create(&threads[0], 0, fp_inherited_control, 0);
  __asm__ volatile("fldcw %0" ::"m"(old_control));
  if (!a)
    pthread_join(threads[0], &first);
  expect("new thread inherits creator floating point environment",
         !a && (u32)first == custom_control, 1);
  struct fp_sample out;
  int child = review_fpu_operation(&samples[0], &out, 2, 0, 0), status = 0;
  if (!child)
    _exit(memcmp(&samples[0], &out, 26) ? 92 : 0);
  expect("fork inherits floating point state without corrupting parent",
         child > 0 && !memcmp(&samples[0], &out, 26) &&
             waitpid(child, &status, 0) == child && status == 0,
         1);
  signal(SIGUSR1, fp_signal);
  review_fpu_operation(&samples[1], &out, 37, getpid(), SIGUSR1);
  expect("signal return restores interrupted floating point state",
         !memcmp(&samples[1], &out, 26), 1);
  signal(SIGUSR1, SIG_DFL);
  child = fork();
  if (!child) {
    u16 control = 0x0b7f;
    __asm__ volatile("fldcw %0" ::"m"(control));
    char *args[] = {"selftest.elf", "--fpu-exec-child", 0};
    execve("/bin/selftest.elf", args, 0);
    _exit(93);
  }
  expect("exec resets floating point environment",
         child > 0 && waitpid(child, &status, 0) == child &&
             WIFEXITED(status) && WEXITSTATUS(status) == 44,
         1);

  int pipes[2];
  pipe(pipes);
  int copy = fcntl(pipes[1], F_DUPFD_CLOEXEC, 100);
  expect("F_DUPFD_CLOEXEC sets flag at descriptor creation",
         copy == 100 && fcntl(copy, F_GETFD, 0) == FD_CLOEXEC &&
             fcntl(pipes[1], F_GETFD, 0) == 0,
         1);
  child = fork();
  if (!child) {
    char *args[] = {"selftest.elf", "--cloexec-child", 0};
    execve("/bin/selftest.elf", args, 0);
    _exit(94);
  }
  expect("atomic CLOEXEC duplicate is closed by exec",
         child > 0 && waitpid(child, &status, 0) == child &&
             WIFEXITED(status) && WEXITSTATUS(status) == 45 &&
             fcntl(copy, F_GETFD, 0) == FD_CLOEXEC,
         1);
  close(copy);
  close(pipes[0]);
  close(pipes[1]);

  u8 image[128] __attribute__((aligned(4)));
  elf_image(image);
  expect("valid minimal ELF executes", run_image(image, 1), 1);
  elf_image(image);
  ((u32 *)image)[15] = ((u32 *)image)[6] = 0x5ff000;
  expect("ELF cannot replace supervisor entry stack", run_image(image, 0), 1);
  elf_image(image);
  ((u32 *)image)[15] = 0x100000;
  expect("ELF cannot map kernel image", run_image(image, 0), 1);
  elf_image(image);
  ((u32 *)image)[15] = 0xfffff000;
  ((u32 *)image)[18] = 8192;
  expect("ELF rejects wrapping virtual range", run_image(image, 0), 1);
  elf_image(image);
  ((u32 *)image)[19] = 6;
  expect("ELF entry must be in executable segment", run_image(image, 0), 1);
  elf_image(image);
  ((u32 *)image)[6] = 0x480000;
  expect("ELF entry must be mapped", run_image(image, 0), 1);
  elf_image(image);
  ((u16 *)image)[9] = 62;
  expect("invalid ELF is rejected instead of raw binary fallback",
         run_image(image, 0), 1);
  elf_image(image);
  ((u16 *)image)[22] = 2;
  memcpy(image + 84, image + 52, 32);
  expect("ELF rejects overlapping load segments", run_image(image, 0), 1);
  unlink("/tmp/review-image");
}
