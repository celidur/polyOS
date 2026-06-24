use crate::interrupts::InterruptFrame;

use super::abi;
use super::file::*;
use super::heap::*;
use super::io::*;
use super::misc::*;
use super::network::*;
use super::process::*;
use super::register::{syscall_get_handler, syscall_register};
use super::signal::*;
use super::sync::*;
use super::types::SyscallId;

pub fn syscall_init() {
    syscall_register(SyscallId::Execve, syscall_execve);
    syscall_register(SyscallId::Fork, syscall_fork);
    syscall_register(SyscallId::Exit, syscall_exit);
    syscall_register(SyscallId::WaitPid, syscall_waitpid);
    syscall_register(SyscallId::GetPid, syscall_getpid);
    syscall_register(SyscallId::GetPpid, syscall_getppid);
    syscall_register(SyscallId::Kill, syscall_kill);
    syscall_register(SyscallId::SigAction, syscall_sigaction);
    syscall_register(SyscallId::SigReturn, syscall_sigreturn);
    syscall_register(SyscallId::PrintMemory, syscall_print_memory);
    syscall_register(SyscallId::Open, syscall_open);
    syscall_register(SyscallId::Read, syscall_read);
    syscall_register(SyscallId::Write, syscall_write);
    syscall_register(SyscallId::Lseek, syscall_lseek);
    syscall_register(SyscallId::Stat, syscall_stat);
    syscall_register(SyscallId::Lstat, syscall_lstat);
    syscall_register(SyscallId::Fstat, syscall_fstat);
    syscall_register(SyscallId::Close, syscall_close);
    syscall_register(SyscallId::Dup, syscall_dup);
    syscall_register(SyscallId::Dup2, syscall_dup2);
    syscall_register(SyscallId::Fcntl, syscall_fcntl);
    syscall_register(SyscallId::Pipe, syscall_pipe);
    syscall_register(SyscallId::Unlink, syscall_unlink);
    syscall_register(SyscallId::Mkdir, syscall_mkdir);
    syscall_register(SyscallId::Rmdir, syscall_rmdir);
    syscall_register(SyscallId::Chdir, syscall_chdir);
    syscall_register(SyscallId::GetCwd, syscall_getcwd);
    syscall_register(SyscallId::GetDents, syscall_getdents);
    syscall_register(SyscallId::Ioctl, syscall_ioctl);
    syscall_register(SyscallId::Brk, syscall_brk);
    syscall_register(SyscallId::NanoSleep, syscall_nanosleep);
    syscall_register(SyscallId::GetTimeOfDay, syscall_gettimeofday);
    syscall_register(SyscallId::ClockGetTime, syscall_clock_gettime);
    syscall_register(SyscallId::LinuxReboot, syscall_linux_reboot);
    syscall_register(SyscallId::NetworkInfo, syscall_network_info);
    syscall_register(
        SyscallId::NetworkDhcpDiscover,
        syscall_network_dhcp_discover,
    );
    syscall_register(SyscallId::NetworkPingGateway, syscall_network_ping_gateway);
    syscall_register(SyscallId::NetworkPingIpv4, syscall_network_ping_ipv4);
    syscall_register(SyscallId::NetworkDnsQuery, syscall_network_dns_query);
    syscall_register(SyscallId::NetworkPingName, syscall_network_ping_name);
    syscall_register(SyscallId::SocketCall, syscall_socketcall);
    syscall_register(
        SyscallId::NetworkRecvFromWait,
        syscall_network_recvfrom_wait,
    );
    syscall_register(SyscallId::SemaphoreCreate, syscall_semaphore_create);
    syscall_register(SyscallId::SemaphoreWait, syscall_semaphore_wait);
    syscall_register(SyscallId::SemaphoreSignal, syscall_semaphore_signal);
    syscall_register(SyscallId::SemaphoreClose, syscall_semaphore_close);
    syscall_register(SyscallId::KernelSelfTest, syscall_kernel_selftest);
}

pub fn syscall_handle(frame: &InterruptFrame) -> u32 {
    let cmd = frame.eax;
    let cmd = match SyscallId::new(cmd) {
        Some(c) => c,
        None => {
            serial_println!("Unknown syscall command: {}", cmd);
            return abi::errno(abi::ENOSYS);
        }
    };

    syscall_get_handler(cmd)
        .map(|handler| handler(frame))
        .unwrap_or_else(|| {
            serial_println!("Unknown syscall command: {:?}", cmd);
            abi::errno(abi::ENOSYS)
        })
}
