#include <errno.h>
#include <polyos.h>
#include <stdio.h>
#include <string.h>
void test_management(void (*expect)(const char *, int, int)) {
  expect("process group lookup", getpgid(0) == getpgrp() && getsid(0) >= 0, 1);
  char name[16];
  expect("prctl thread name",
         prctl(PR_SET_NAME, (long)"runtime-test") == 0 &&
             prctl(PR_GET_NAME, (long)name) == 0 &&
             strncmp(name, "runtime-test", 16) == 0,
         1);
  expect("no_new_privs monotonic flag",
         prctl(PR_SET_NO_NEW_PRIVS, 1L, 0L, 0L, 0L) == 0 &&
             prctl(PR_GET_NO_NEW_PRIVS, 0L, 0L, 0L, 0L) == 1,
         1);
  int child = fork(), status = 0;
  if (child == 0) {
    pid_t sid = setsid();
    _exit(sid == getpid() && getsid(0) == sid && getpgrp() == sid &&
                  setsid() == -1
              ? 0
              : 91);
  }
  expect("setsid creates session",
         waitpid(child, &status, 0) == child && status == 0, 1);
  child = fork();
  if (child == 0) {
    for (;;)
      pause();
  }
  expect("setpgid child", setpgid(child, child) == 0 && getpgid(child) == child,
         1);
  killpg(child, SIGSTOP);
  expect("waitpid reports stopped group",
         waitpid(-child, &status, WUNTRACED) == child && WIFSTOPPED(status), 1);
  killpg(child, SIGCONT);
  expect("waitpid reports continued group",
         waitpid(-child, &status, WCONTINUED) == child && WIFCONTINUED(status),
         1);
  killpg(child, SIGTERM);
  expect("waitpid selects process group",
         waitpid(-child, &status, 0) == child && WIFSIGNALED(status), 1);
  child = fork();
  if (child == 0)
    _exit(37);
  siginfo_t info;
  expect("waitid exit details",
         waitid(P_ALL, 0, &info, WEXITED) == 0 && info.si_pid == child &&
             info.si_code == CLD_EXITED && info.si_status == 37,
         1);
  child = fork();
  if (child == 0) {
    gid_t groups[] = {123, 456};
    if (setgroups(2, groups) || setgid(123) || setuid(1000))
      _exit(90);
    gid_t got[2];
    if (getuid() != 1000 || geteuid() != 1000 || getgid() != 123 ||
        getgroups(2, got) != 2 || got[1] != 456)
      _exit(89);
    errno = 0;
    if (setuid(0) != -1 || errno != EPERM)
      _exit(88);
    errno = 0;
    if (kill(getppid(), 0) != -1 || errno != EPERM)
      _exit(87);
    _exit(0);
  }
  expect("credential drop and signal permissions",
         waitpid(child, &status, 0) == child && status == 0, 1);
  mkdir("/tmp/jail", 0700);
  int fd = open("/tmp/jail/inside", O_CREAT | O_RDWR, 0600);
  close(fd);
  child = fork();
  if (child == 0) {
    if (chroot("/tmp/jail"))
      _exit(86);
    char cwd[16];
    fd = open("/inside", 0, 0);
    if (fd < 0 || !getcwd(cwd, sizeof(cwd)) || strncmp(cwd, "/", 16))
      _exit(85);
    close(fd);
    if (chdir("../../") || open("/bin/selftest.elf", 0, 0) != -1)
      _exit(84);
    _exit(0);
  }
  expect("chroot path confinement",
         waitpid(child, &status, 0) == child && status == 0, 1);
  unlink("/tmp/jail/inside");
  rmdir("/tmp/jail");
  child = fork();
  if (child == 0) {
    char *args[] = {"selftest.elf", "--thread-exec-child", 0};
    execvp("selftest.elf", args);
    _exit(83);
  }
  expect("execvp PATH search",
         waitpid(child, &status, 0) == child && WIFEXITED(status) &&
             WEXITSTATUS(status) == 43,
         1);

  posix_spawn_file_actions_t actions;
  posix_spawn_file_actions_init(&actions);
  posix_spawn_file_actions_addopen(&actions, 9, "/tmp/spawn-output",
                                   O_CREAT | O_RDWR | O_TRUNC, 0600);
  posix_spawn_file_actions_adddup2(&actions, 9, 1);
  posix_spawn_file_actions_addclose(&actions, 9);
  char *argv[] = {"selftest.elf", "--thread-exec-child", 0};
  extern char **environ;
  pid_t spawned = -1;
  int error =
      posix_spawn(&spawned, "/bin/selftest.elf", &actions, 0, argv, environ);
  expect("posix_spawn applies file actions",
         error == 0 && waitpid(spawned, &status, 0) == spawned &&
             WEXITSTATUS(status) == 43,
         1);
  posix_spawn_file_actions_destroy(&actions);
  unlink("/tmp/spawn-output");
  spawned = -1;
  expect("posix_spawn reports exec failure",
         posix_spawn(&spawned, "/missing-spawn-program", 0, 0, argv, environ) ==
                 ENOENT &&
             spawned == -1,
         1);
  expect("posix_spawnp PATH search",
         posix_spawnp(&spawned, "selftest.elf", 0, 0, argv, environ) == 0 &&
             waitpid(spawned, &status, 0) == spawned &&
             WEXITSTATUS(status) == 43,
         1);

  child = fork();
  if (child == 0) {
    if (chroot("/proc"))
      _exit(82);
    int escaped = open("/self/exe", 0, 0);
    _exit(escaped == -1 ? 0 : 81);
  }
  expect("chroot confines absolute links",
         waitpid(child, &status, 0) == child && status == 0, 1);

  child = fork();
  if (child == 0) {
    setuid(1000);
    char *args[] = {"renamed", "--credentials-child", 0};
    execve("/bin/selftest.elf", args, environ);
    _exit(80);
  }
  expect("exec preserves credentials and auxiliary IDs",
         waitpid(child, &status, 0) == child && WEXITSTATUS(status) == 48, 1);

  child = fork();
  if (child == 0) {
    setuid(1000);
    _exit(38);
  }
  expect("waitid reports actual child UID",
         waitid(P_PID, child, &info, WEXITED) == 0 && info.si_uid == 1000 &&
             info.si_status == 38,
         1);
}
