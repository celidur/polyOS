use super::thread;
use crate::interrupts::InterruptFrame;

use super::abi;
use super::file::*;
use super::heap::*;
use super::io::*;
use super::misc::*;
use super::network::*;
use super::process::*;
use super::register::{SyscallHandler, syscall_get_handler, syscall_register};
use super::signal::*;
use super::sync::*;
use super::types::SyscallId;

pub fn syscall_init() {
    syscall_register(
        SyscallId::Clone,
        SyscallHandler::Args5(thread::syscall_clone),
    );
    syscall_register(
        SyscallId::GetTid,
        SyscallHandler::Args0(thread::syscall_gettid),
    );
    syscall_register(
        SyscallId::ExitGroup,
        SyscallHandler::Args1(thread::syscall_exit_group),
    );
    syscall_register(
        SyscallId::SchedYield,
        SyscallHandler::Args0(thread::syscall_sched_yield),
    );
    syscall_register(
        SyscallId::Futex,
        SyscallHandler::Args4(thread::syscall_futex),
    );

    syscall_register(SyscallId::Execve, SyscallHandler::Args3(syscall_execve));
    syscall_register(SyscallId::Fork, SyscallHandler::Args0(syscall_fork));
    syscall_register(SyscallId::Exit, SyscallHandler::Args1(thread::syscall_exit));
    syscall_register(SyscallId::WaitPid, SyscallHandler::Args3(syscall_waitpid));
    syscall_register(SyscallId::GetPid, SyscallHandler::Args0(syscall_getpid));
    syscall_register(SyscallId::GetUid, SyscallHandler::Args0(syscall_getuid));
    syscall_register(SyscallId::GetPpid, SyscallHandler::Args0(syscall_getppid));
    syscall_register(SyscallId::GetGid, SyscallHandler::Args0(syscall_getgid));
    syscall_register(SyscallId::GetEuid, SyscallHandler::Args0(syscall_geteuid));
    syscall_register(SyscallId::GetEgid, SyscallHandler::Args0(syscall_getegid));
    syscall_register(SyscallId::Kill, SyscallHandler::Args2(syscall_kill));
    syscall_register(
        SyscallId::SigAction,
        SyscallHandler::Args3(syscall_sigaction),
    );
    syscall_register(
        SyscallId::SigReturn,
        SyscallHandler::Args1(syscall_sigreturn),
    );
    syscall_register(
        SyscallId::PrintMemory,
        SyscallHandler::Args0(syscall_print_memory),
    );
    syscall_register(SyscallId::Open, SyscallHandler::Args3(syscall_open));
    syscall_register(SyscallId::Read, SyscallHandler::Args3(syscall_read));
    syscall_register(SyscallId::Write, SyscallHandler::Args3(syscall_write));
    syscall_register(SyscallId::Lseek, SyscallHandler::Args3(syscall_lseek));
    syscall_register(SyscallId::Stat, SyscallHandler::Args2(syscall_stat));
    syscall_register(SyscallId::Lstat, SyscallHandler::Args2(syscall_lstat));
    syscall_register(SyscallId::Fstat, SyscallHandler::Args2(syscall_fstat));
    syscall_register(SyscallId::Close, SyscallHandler::Args1(syscall_close));
    syscall_register(SyscallId::Dup, SyscallHandler::Args1(syscall_dup));
    syscall_register(SyscallId::Dup2, SyscallHandler::Args2(syscall_dup2));
    syscall_register(SyscallId::Fcntl, SyscallHandler::Args3(syscall_fcntl));
    syscall_register(SyscallId::Pipe, SyscallHandler::Args1(syscall_pipe));
    syscall_register(SyscallId::Unlink, SyscallHandler::Args1(syscall_unlink));
    syscall_register(SyscallId::Chmod, SyscallHandler::Args2(syscall_chmod));
    syscall_register(SyscallId::Mkdir, SyscallHandler::Args2(syscall_mkdir));
    syscall_register(SyscallId::Rmdir, SyscallHandler::Args1(syscall_rmdir));
    syscall_register(SyscallId::Umask, SyscallHandler::Args1(syscall_umask));
    syscall_register(SyscallId::Chdir, SyscallHandler::Args1(syscall_chdir));
    syscall_register(SyscallId::Chown, SyscallHandler::Args3(syscall_chown));
    syscall_register(SyscallId::GetCwd, SyscallHandler::Args2(syscall_getcwd));
    syscall_register(SyscallId::GetDents, SyscallHandler::Args3(syscall_getdents));
    syscall_register(SyscallId::Ioctl, SyscallHandler::Args3(syscall_ioctl));
    syscall_register(SyscallId::Brk, SyscallHandler::Args1(syscall_brk));
    syscall_register(
        SyscallId::NanoSleep,
        SyscallHandler::Args2(syscall_nanosleep),
    );
    syscall_register(
        SyscallId::GetTimeOfDay,
        SyscallHandler::Args2(syscall_gettimeofday),
    );
    syscall_register(
        SyscallId::ClockGetTime,
        SyscallHandler::Args2(syscall_clock_gettime),
    );
    syscall_register(
        SyscallId::LinuxReboot,
        SyscallHandler::Args3(syscall_linux_reboot),
    );
    syscall_register(
        SyscallId::SocketCall,
        SyscallHandler::Args2(syscall_socketcall),
    );
    syscall_register(
        SyscallId::SemaphoreCreate,
        SyscallHandler::Args1(syscall_semaphore_create),
    );
    syscall_register(
        SyscallId::SemaphoreWait,
        SyscallHandler::Args1(syscall_semaphore_wait),
    );
    syscall_register(
        SyscallId::SemaphoreSignal,
        SyscallHandler::Args1(syscall_semaphore_signal),
    );
    syscall_register(
        SyscallId::SemaphoreClose,
        SyscallHandler::Args1(syscall_semaphore_close),
    );
    syscall_register(
        SyscallId::KernelSelfTest,
        SyscallHandler::Args0(syscall_kernel_selftest),
    );
}

pub fn syscall_handle(frame: &InterruptFrame) -> u32 {
    let cmd = frame.syscall_number();
    let cmd = match SyscallId::new(cmd) {
        Some(c) => c,
        None => {
            serial_println!("Unknown syscall command: {}", cmd);
            return abi::errno(abi::ENOSYS);
        }
    };

    syscall_get_handler(cmd)
        .map(|handler| handler.call(frame))
        .unwrap_or_else(|| {
            serial_println!("Unknown syscall command: {:?}", cmd);
            abi::errno(abi::ENOSYS)
        })
}
