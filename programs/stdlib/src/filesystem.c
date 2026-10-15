#include "syscall_internal.h"
#include <polyos.h>
extern int __sys_getdents(int fd, struct dirent *dirp, size_t count);
extern int __sys_getcwd(char *buf, size_t size);
extern int __sys_chown(const char *pathname, unsigned int uid,
                       unsigned int gid);
extern int __sys_chdir(const char *pathname);
extern int __sys_umask(int mask);
extern int __sys_rmdir(const char *pathname);
extern int __sys_mkdir(const char *pathname, int mode);
extern int __sys_chmod(const char *pathname, int mode);
extern int __sys_unlink(const char *pathname);
extern int __sys_dup2(int oldfd, int newfd);
extern int __sys_dup(int oldfd);
extern int __sys_close(int fd);
extern int __sys_fcntl(int fd, int cmd, long arg);
extern int __sys_fstat(int fd, struct file_stat *stat);
extern int __sys_lstat(const char *pathname, struct file_stat *stat);
extern int __sys_stat(const char *pathname, struct file_stat *stat);
extern off_t __sys_lseek(int fd, off_t offset, int whence);
extern ssize_t __sys_write(int fd, const void *buf, size_t count);
extern ssize_t __sys_read(int fd, void *buf, size_t count);
extern int __sys_open(const char *pathname, int flags, int mode);
int openat(int fd, const char *path, int flags, int mode) {
  return call6(295, fd, (long)path, flags, mode, 0, 0);
}
int ftruncate(int fd, off_t length) {
  return call6(93, fd, length, 0, 0, 0, 0);
}
int fchmod(int fd, int mode) { return call6(94, fd, mode, 0, 0, 0, 0); }
int access(const char *path, int mode) {
  return call6(33, (long)path, mode, 0, 0, 0, 0);
}
ssize_t readlink(const char *path, char *buf, size_t size) {
  return call6(85, (long)path, (long)buf, size, 0, 0, 0);
}
int fstatat64(int fd, const char *path, struct stat64 *out, int flags) {
  return call6(300, fd, (long)path, (long)out, flags, 0, 0);
}
int stat64(const char *path, struct stat64 *out) {
  return call6(195, (long)path, (long)out, 0, 0, 0, 0);
}
int lstat64(const char *path, struct stat64 *out) {
  return call6(196, (long)path, (long)out, 0, 0, 0, 0);
}
int fstat64(int fd, struct stat64 *out) {
  return call6(197, fd, (long)out, 0, 0, 0, 0);
}
int getdents64(int fd, void *buf, size_t len) {
  return call6(220, fd, (long)buf, len, 0, 0, 0);
}
int fstatat(int fd, const char *path, struct file_stat *out, int flags) {
  if (!out) {
    errno = EFAULT;
    return -1;
  }
  struct stat64 st;
  int result = fstatat64(fd, path, &st, flags);
  if (result < 0)
    return result;
  *out = (struct file_stat){.size = (int)st.st_size,
                            .flags = 0,
                            .mode = st.st_mode,
                            .uid = st.st_uid,
                            .gid = st.st_gid,
                            .is_dir = S_ISDIR(st.st_mode)};
  return 0;
}

int open(const char *pathname, int flags, int mode) {
  return syscall_result(__sys_open(pathname, flags, mode));
}

ssize_t read(int fd, void *buf, size_t count) {
  return syscall_result(__sys_read(fd, buf, count));
}

ssize_t write(int fd, const void *buf, size_t count) {
  return syscall_result(__sys_write(fd, buf, count));
}

off_t lseek(int fd, off_t offset, int whence) {
  return syscall_result(__sys_lseek(fd, offset, whence));
}

int fstat(int fd, struct file_stat *stat) {
  return syscall_result(__sys_fstat(fd, stat));
}

int stat(const char *pathname, struct file_stat *stat) {
  return syscall_result(__sys_stat(pathname, stat));
}

int lstat(const char *pathname, struct file_stat *stat) {
  return syscall_result(__sys_lstat(pathname, stat));
}

int close(int fd) { return syscall_result(__sys_close(fd)); }

int fcntl(int fd, int cmd, long arg) {
  return syscall_result(__sys_fcntl(fd, cmd, arg));
}

int dup(int oldfd) { return syscall_result(__sys_dup(oldfd)); }

int dup2(int oldfd, int newfd) {
  return syscall_result(__sys_dup2(oldfd, newfd));
}

int unlink(const char *pathname) {
  return syscall_result(__sys_unlink(pathname));
}

int chmod(const char *pathname, int mode) {
  return syscall_result(__sys_chmod(pathname, mode));
}

int mkdir(const char *pathname, int mode) {
  return syscall_result(__sys_mkdir(pathname, mode));
}

int rmdir(const char *pathname) {
  return syscall_result(__sys_rmdir(pathname));
}

int umask(int mask) { return __sys_umask(mask); }

int chdir(const char *pathname) {
  return syscall_result(__sys_chdir(pathname));
}

int chown(const char *pathname, unsigned int uid, unsigned int gid) {
  return syscall_result(__sys_chown(pathname, uid, gid));
}

char *getcwd(char *buf, size_t size) {
  int result = __sys_getcwd(buf, size);
  if (result < 0 && result >= -4095) {
    errno = -result;
    return NULL;
  }

  return buf;
}

int getdents(int fd, struct dirent *dirp, size_t count) {
  return syscall_result(__sys_getdents(fd, dirp, count));
}
