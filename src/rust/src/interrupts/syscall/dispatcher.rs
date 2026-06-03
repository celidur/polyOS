use crate::interrupts::InterruptFrame;

use super::abi;
use super::file::*;
use super::heap::*;
use super::io::*;
use super::misc::*;
use super::network::*;
use super::process::*;
use super::register::{syscall_get_handler, syscall_register};
use super::sync::*;
use super::types::SyscallId;

pub fn syscall_init() {
    syscall_register(SyscallId::Sleep, syscall_sleep);
    syscall_register(SyscallId::Malloc, syscall_malloc);
    syscall_register(SyscallId::Free, syscall_free);
    syscall_register(SyscallId::Execve, syscall_execve);
    syscall_register(SyscallId::Fork, syscall_fork);
    syscall_register(SyscallId::Exit, syscall_exit);
    syscall_register(SyscallId::WaitPid, syscall_waitpid);
    syscall_register(SyscallId::GetPid, syscall_getpid);
    syscall_register(SyscallId::GetPpid, syscall_getppid);
    syscall_register(SyscallId::PrintMemory, syscall_print_memory);
    syscall_register(SyscallId::Open, syscall_open);
    syscall_register(SyscallId::Read, syscall_read);
    syscall_register(SyscallId::Write, syscall_write);
    syscall_register(SyscallId::Lseek, syscall_lseek);
    syscall_register(SyscallId::Fstat, syscall_fstat);
    syscall_register(SyscallId::Close, syscall_close);
    syscall_register(SyscallId::Pipe, syscall_pipe);
    syscall_register(SyscallId::Ioctl, syscall_ioctl);
    syscall_register(SyscallId::Reboot, syscall_reboot);
    syscall_register(SyscallId::Shutdown, syscall_shutdown);
    syscall_register(SyscallId::NetworkInfo, syscall_network_info);
    syscall_register(
        SyscallId::NetworkDhcpDiscover,
        syscall_network_dhcp_discover,
    );
    syscall_register(SyscallId::NetworkPingGateway, syscall_network_ping_gateway);
    syscall_register(SyscallId::NetworkPingIpv4, syscall_network_ping_ipv4);
    syscall_register(SyscallId::NetworkDnsQuery, syscall_network_dns_query);
    syscall_register(SyscallId::NetworkPingName, syscall_network_ping_name);
    syscall_register(SyscallId::NetworkSocket, syscall_network_socket);
    syscall_register(SyscallId::NetworkSendTo, syscall_network_sendto);
    syscall_register(SyscallId::NetworkRecvFrom, syscall_network_recvfrom);
    syscall_register(
        SyscallId::NetworkRecvFromWait,
        syscall_network_recvfrom_wait,
    );
    syscall_register(SyscallId::SemaphoreCreate, syscall_semaphore_create);
    syscall_register(SyscallId::SemaphoreWait, syscall_semaphore_wait);
    syscall_register(SyscallId::SemaphoreSignal, syscall_semaphore_signal);
    syscall_register(SyscallId::SemaphoreClose, syscall_semaphore_close);
}

pub fn syscall_handle(frame: &InterruptFrame) -> u32 {
    let cmd = frame.eax;
    let cmd = match SyscallId::new(cmd as u8) {
        Some(c) => c,
        None => {
            serial_println!("Unknown syscall command: {}", cmd);
            return abi::error();
        }
    };

    syscall_get_handler(cmd)
        .map(|handler| handler(frame))
        .unwrap_or_else(|| {
            serial_println!("Unknown syscall command: {:?}", cmd);
            abi::error()
        })
}
