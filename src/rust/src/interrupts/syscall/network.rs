use crate::{
    interrupts::InterruptFrame,
    kernel::KERNEL,
    net,
    schedule::process::{Process, ProcessDescriptor, SocketHandle},
};
use alloc::sync::Arc;
use spin::Mutex;

use super::{abi, user};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct NetworkStat {
    pub present: u32,
    pub dhcp_state: u32,
    pub mac: [u8; 6],
    pub _padding: [u8; 2],
    pub ipv4: [u8; 4],
    pub subnet_mask: [u8; 4],
    pub router: [u8; 4],
    pub dns: [u8; 4],
    pub packets_rx: u64,
    pub packets_tx: u64,
    pub arp_entries: u32,
    pub ping_tx: u32,
    pub ping_rx: u32,
    pub dns_tx: u32,
    pub dns_rx: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct SockAddrIn {
    sin_family: u16,
    sin_port: u16,
    sin_addr: u32,
    sin_zero: [u8; 8],
}

#[derive(Clone, Copy)]
struct RecvFromArgs {
    socket_id: u32,
    buf_ptr: u32,
    len: u32,
    src_ptr: u32,
    addrlen_ptr: u32,
}

const SOCKETCALL_SOCKET: u32 = 1;
const SOCKETCALL_BIND: u32 = 2;
const SOCKETCALL_CONNECT: u32 = 3;
const SOCKETCALL_LISTEN: u32 = 4;
const SOCKETCALL_ACCEPT: u32 = 5;
const SOCKETCALL_GETSOCKNAME: u32 = 6;
const SOCKETCALL_GETPEERNAME: u32 = 7;
const SOCKETCALL_SEND: u32 = 9;
const SOCKETCALL_RECV: u32 = 10;
const SOCKETCALL_SENDTO: u32 = 11;
const SOCKETCALL_RECVFROM: u32 = 12;
const SOCKETCALL_SETSOCKOPT: u32 = 14;

pub fn syscall_network_info(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return abi::errno(abi::ESRCH);
        };

        let ptr = current_task.read().get_stack_item(0);
        if ptr == 0 {
            return abi::errno(abi::EFAULT);
        }

        let mut stat = NetworkStat::default();
        if let Some(info) = net::info() {
            stat.present = 1;
            stat.dhcp_state = info.dhcp_state as u32;
            stat.mac = info.mac;
            stat.ipv4 = info.ipv4.address;
            stat.subnet_mask = info.ipv4.subnet_mask;
            stat.router = info.ipv4.router;
            stat.dns = info.ipv4.dns;
            stat.packets_rx = info.packets_rx;
            stat.packets_tx = info.packets_tx;
            stat.arp_entries = info.arp_entries;
            stat.ping_tx = info.ping_tx;
            stat.ping_rx = info.ping_rx;
            stat.dns_tx = info.dns_tx;
            stat.dns_rx = info.dns_rx;
        }

        if user::write_value(&current_task.read().process.page_directory, ptr, &stat).is_err() {
            return abi::errno(abi::EFAULT);
        }

        0
    })
}

pub fn syscall_network_dhcp_discover(_frame: &InterruptFrame) -> u32 {
    match net::send_dhcp_discover() {
        Ok(()) => 0,
        Err(error) => {
            serial_println!("net: dhcp discover failed: {:?}", error);
            network_errno(error)
        }
    }
}

pub fn syscall_network_ping_gateway(_frame: &InterruptFrame) -> u32 {
    match net::ping_gateway() {
        Ok(()) => 0,
        Err(error) => {
            serial_println!("net: ping gateway failed: {:?}", error);
            network_errno(error)
        }
    }
}

pub fn syscall_network_ping_ipv4(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return abi::errno(abi::ESRCH);
        };

        let packed_ip = current_task.read().get_stack_item(0);
        let target_ip = [
            ((packed_ip >> 24) & 0xff) as u8,
            ((packed_ip >> 16) & 0xff) as u8,
            ((packed_ip >> 8) & 0xff) as u8,
            (packed_ip & 0xff) as u8,
        ];

        match net::ping_ipv4(target_ip) {
            Ok(()) => 0,
            Err(error) => {
                serial_println!(
                    "net: ping {}.{}.{}.{} failed: {:?}",
                    target_ip[0],
                    target_ip[1],
                    target_ip[2],
                    target_ip[3],
                    error
                );
                network_errno(error)
            }
        }
    })
}

pub fn syscall_network_dns_query(_frame: &InterruptFrame) -> u32 {
    syscall_network_name_request(net::send_dns_query, "dns query")
}

pub fn syscall_network_ping_name(_frame: &InterruptFrame) -> u32 {
    syscall_network_name_request(net::ping_name, "ping")
}

pub fn syscall_socketcall(_frame: &InterruptFrame) -> u32 {
    let (process, call, args_ptr) = match KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some((
            task.process.clone(),
            task.get_stack_item(0),
            task.get_stack_item(1),
        ))
    }) {
        Some(values) => values,
        None => return abi::errno(abi::ESRCH),
    };

    match call {
        SOCKETCALL_SOCKET => {
            let args = match read_socketcall_args(&process, args_ptr, 3) {
                Ok(args) => args,
                Err(errno) => return abi::errno(errno),
            };
            socket_open_for_process(&process, args[0], args[1], args[2])
        }
        SOCKETCALL_BIND => {
            let args = match read_socketcall_args(&process, args_ptr, 3) {
                Ok(args) => args,
                Err(errno) => return abi::errno(errno),
            };
            socket_bind_for_process(&process, args[0], args[1], args[2])
        }
        SOCKETCALL_CONNECT => {
            let args = match read_socketcall_args(&process, args_ptr, 3) {
                Ok(args) => args,
                Err(errno) => return abi::errno(errno),
            };
            socket_connect_for_process(&process, args[0], args[1], args[2])
        }
        SOCKETCALL_LISTEN => {
            let args = match read_socketcall_args(&process, args_ptr, 2) {
                Ok(args) => args,
                Err(errno) => return abi::errno(errno),
            };
            socket_listen_for_process(&process, args[0], args[1])
        }
        SOCKETCALL_ACCEPT => {
            let args = match read_socketcall_args(&process, args_ptr, 3) {
                Ok(args) => args,
                Err(errno) => return abi::errno(errno),
            };
            socket_accept_for_process(&process, args[0], args[1], args[2])
        }
        SOCKETCALL_GETSOCKNAME => {
            let args = match read_socketcall_args(&process, args_ptr, 3) {
                Ok(args) => args,
                Err(errno) => return abi::errno(errno),
            };
            socket_sockaddr_query_for_process(
                &process,
                args[0],
                args[1],
                args[2],
                net::socket_local_addr,
            )
        }
        SOCKETCALL_GETPEERNAME => {
            let args = match read_socketcall_args(&process, args_ptr, 3) {
                Ok(args) => args,
                Err(errno) => return abi::errno(errno),
            };
            socket_sockaddr_query_for_process(
                &process,
                args[0],
                args[1],
                args[2],
                net::socket_peer_addr,
            )
        }
        SOCKETCALL_SEND => {
            let args = match read_socketcall_args(&process, args_ptr, 4) {
                Ok(args) => args,
                Err(errno) => return abi::errno(errno),
            };
            socket_send_for_process(&process, args[0], args[1], args[2], args[3])
        }
        SOCKETCALL_RECV => {
            let args = match read_socketcall_args(&process, args_ptr, 4) {
                Ok(args) => args,
                Err(errno) => return abi::errno(errno),
            };
            socket_recv_for_process(&process, args[0], args[1], args[2], args[3])
        }
        SOCKETCALL_SENDTO => {
            let args = match read_socketcall_args(&process, args_ptr, 6) {
                Ok(args) => args,
                Err(errno) => return abi::errno(errno),
            };
            socket_sendto_for_process(
                &process, args[0], args[1], args[2], args[3], args[4], args[5],
            )
        }
        SOCKETCALL_RECVFROM => {
            let args = match read_socketcall_args(&process, args_ptr, 6) {
                Ok(args) => args,
                Err(errno) => return abi::errno(errno),
            };
            socket_recvfrom_for_process(
                &process, args[0], args[1], args[2], args[3], args[4], args[5],
            )
        }
        SOCKETCALL_SETSOCKOPT => {
            let args = match read_socketcall_args(&process, args_ptr, 5) {
                Ok(args) => args,
                Err(errno) => return abi::errno(errno),
            };
            socket_setsockopt_for_process(&process, args[0], args[1], args[2], args[3], args[4])
        }
        _ => abi::errno(abi::EINVAL),
    }
}

pub fn syscall_network_recvfrom_wait(_frame: &InterruptFrame) -> u32 {
    let args = match read_recvfrom_wait_args() {
        Ok(args) => args,
        Err(errno) => return abi::errno(errno),
    };
    let socket_id = match socket_id_from_fd(args.socket_id) {
        Ok(socket_id) => socket_id,
        Err(errno) => return abi::errno(errno),
    };

    let timeout_ticks = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        Some(current_task.read().get_stack_item(6) as u64)
    });

    let Some(timeout_ticks) = timeout_ticks else {
        return abi::errno(abi::ESRCH);
    };

    let start_tick = KERNEL.with_task_manager(|tm| tm.get_tick());

    loop {
        match net::socket_recv_from(socket_id, args.len as usize) {
            Ok(packet) => return write_recvfrom_result(args, packet),
            Err(net::NetworkError::WouldBlock) => {
                if timeout_ticks != 0 {
                    let now = KERNEL.with_task_manager(|tm| tm.get_tick());
                    if now.saturating_sub(start_tick) >= timeout_ticks {
                        return abi::WAIT_TIMEOUT;
                    }
                }

                core::hint::spin_loop();
            }
            Err(error) => {
                serial_println!("net: recvfrom_wait({}) failed: {:?}", args.socket_id, error);
                return network_errno(error);
            }
        }
    }
}

fn syscall_network_name_request(
    request: fn(&str) -> Result<(), net::NetworkError>,
    label: &str,
) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return abi::errno(abi::ESRCH);
        };

        let task = current_task.read();
        let name_ptr = task.get_stack_item(0);
        if name_ptr == 0 {
            return abi::errno(abi::EFAULT);
        }

        let Some(name) = user::read_c_string(&task, name_ptr, 256) else {
            return abi::errno(abi::EFAULT);
        };

        match request(name.as_str()) {
            Ok(()) => 0,
            Err(error) => {
                serial_println!("net: {} {} failed: {:?}", label, name, error);
                network_errno(error)
            }
        }
    })
}

fn socket_id_from_fd(fd: u32) -> Result<u32, i32> {
    KERNEL.with_task_manager(|tm| {
        let Some(current_task) = tm.get_current() else {
            return Err(abi::ESRCH);
        };
        let process = current_task.read().process.clone();
        socket_id_from_process_fd(&process, fd)
    })
}

fn socket_id_from_process_fd(process: &Process, fd: u32) -> Result<u32, i32> {
    match process.get_fd(fd as i32) {
        Some(ProcessDescriptor::Socket(socket)) => Ok(socket.lock().id()),
        _ => Err(abi::EBADF),
    }
}

fn read_socketcall_args(process: &Process, args_ptr: u32, count: usize) -> Result<[u32; 6], i32> {
    if args_ptr == 0 || count > 6 {
        return Err(abi::EFAULT);
    }

    let mut args = [0_u32; 6];
    user::copy_from_user(
        &process.page_directory,
        args_ptr,
        args.as_mut_ptr() as *mut u8,
        (count * core::mem::size_of::<u32>()) as u32,
    )
    .map_err(|_| abi::EFAULT)?;

    Ok(args)
}

fn socket_open_for_process(process: &Process, domain: u32, socket_type: u32, protocol: u32) -> u32 {
    match net::socket_open(domain, socket_type, protocol) {
        Ok(socket_id) => match process.insert_fd(ProcessDescriptor::Socket(Arc::new(Mutex::new(
            SocketHandle::new(socket_id),
        )))) {
            Ok(fd) => fd as u32,
            Err(error) => {
                let _ = net::socket_close(socket_id);
                serial_println!("net: socket fd allocation failed: {:?}", error);
                abi::errno(abi::EMFILE)
            }
        },
        Err(error) => {
            serial_println!(
                "net: socket({}, {}, {}) failed: {:?}",
                domain,
                socket_type,
                protocol,
                error
            );
            network_errno(error)
        }
    }
}

fn socket_sendto_for_process(
    process: &Process,
    fd: u32,
    _buf_ptr: u32,
    len: u32,
    _flags: u32,
    dest_ptr: u32,
    dest_len: u32,
) -> u32 {
    let dest = match read_sockaddr_in(process, dest_ptr, dest_len) {
        Ok(dest) => dest,
        Err(errno) => return abi::errno(errno),
    };
    let (target_ip, target_port) = sockaddr_ip_port(dest);
    let socket_id = match socket_id_from_process_fd(process, fd) {
        Ok(socket_id) => socket_id,
        Err(errno) => return abi::errno(errno),
    };

    match net::socket_send_to(socket_id, len as usize, target_ip, target_port) {
        Ok(sent) => sent as u32,
        Err(error) => {
            serial_println!("net: sendto({}) failed: {:?}", fd, error);
            network_errno(error)
        }
    }
}

fn socket_bind_for_process(process: &Process, fd: u32, addr_ptr: u32, addr_len: u32) -> u32 {
    let socket_id = match socket_id_from_process_fd(process, fd) {
        Ok(socket_id) => socket_id,
        Err(errno) => return abi::errno(errno),
    };
    let addr = match read_sockaddr_in(process, addr_ptr, addr_len) {
        Ok(addr) => addr,
        Err(errno) => return abi::errno(errno),
    };
    let (local_ip, local_port) = sockaddr_ip_port(addr);

    match net::socket_bind(socket_id, local_ip, local_port) {
        Ok(()) => 0,
        Err(error) => network_errno(error),
    }
}

fn socket_connect_for_process(process: &Process, fd: u32, addr_ptr: u32, addr_len: u32) -> u32 {
    let socket_id = match socket_id_from_process_fd(process, fd) {
        Ok(socket_id) => socket_id,
        Err(errno) => return abi::errno(errno),
    };
    let addr = match read_sockaddr_in(process, addr_ptr, addr_len) {
        Ok(addr) => addr,
        Err(errno) => return abi::errno(errno),
    };
    let (peer_ip, peer_port) = sockaddr_ip_port(addr);

    match net::socket_connect(socket_id, peer_ip, peer_port) {
        Ok(()) => 0,
        Err(error) => network_errno(error),
    }
}

fn socket_listen_for_process(process: &Process, fd: u32, _backlog: u32) -> u32 {
    match socket_id_from_process_fd(process, fd) {
        Ok(_) => abi::errno(abi::ENOTSUP),
        Err(errno) => abi::errno(errno),
    }
}

fn socket_accept_for_process(process: &Process, fd: u32, _addr_ptr: u32, _addrlen_ptr: u32) -> u32 {
    match socket_id_from_process_fd(process, fd) {
        Ok(_) => abi::errno(abi::ENOTSUP),
        Err(errno) => abi::errno(errno),
    }
}

fn socket_send_for_process(process: &Process, fd: u32, buf_ptr: u32, len: u32, _flags: u32) -> u32 {
    if len != 0 && buf_ptr == 0 {
        return abi::errno(abi::EFAULT);
    }
    let socket_id = match socket_id_from_process_fd(process, fd) {
        Ok(socket_id) => socket_id,
        Err(errno) => return abi::errno(errno),
    };

    match net::socket_send(socket_id, len as usize) {
        Ok(sent) => sent as u32,
        Err(error) => network_errno(error),
    }
}

fn socket_recv_for_process(process: &Process, fd: u32, buf_ptr: u32, len: u32, _flags: u32) -> u32 {
    if len == 0 {
        return 0;
    }
    if buf_ptr == 0 {
        return abi::errno(abi::EFAULT);
    }

    let socket_id = match socket_id_from_process_fd(process, fd) {
        Ok(socket_id) => socket_id,
        Err(errno) => return abi::errno(errno),
    };

    let packet = match net::socket_recv_from(socket_id, len as usize) {
        Ok(packet) => packet,
        Err(error) => return network_errno(error),
    };
    let read_len = packet.data.len() as u32;
    if user::copy_to_user(
        &process.page_directory,
        buf_ptr,
        packet.data.as_ptr(),
        read_len,
    )
    .is_err()
    {
        return abi::errno(abi::EFAULT);
    }

    read_len
}

fn socket_recvfrom_for_process(
    process: &Process,
    fd: u32,
    buf_ptr: u32,
    len: u32,
    _flags: u32,
    src_ptr: u32,
    addrlen_ptr: u32,
) -> u32 {
    if buf_ptr == 0 {
        return abi::errno(abi::EFAULT);
    }
    if len == 0 {
        return abi::errno(abi::EINVAL);
    }

    let socket_id = match socket_id_from_process_fd(process, fd) {
        Ok(socket_id) => socket_id,
        Err(errno) => return abi::errno(errno),
    };
    let packet = match net::socket_recv_from(socket_id, len as usize) {
        Ok(packet) => packet,
        Err(error) => return network_errno(error),
    };

    write_recvfrom_result_for_process(
        process,
        RecvFromArgs {
            socket_id: fd,
            buf_ptr,
            len,
            src_ptr,
            addrlen_ptr,
        },
        packet,
    )
}

fn socket_sockaddr_query_for_process(
    process: &Process,
    fd: u32,
    addr_ptr: u32,
    addrlen_ptr: u32,
    query: fn(u32) -> Result<([u8; 4], u16), net::NetworkError>,
) -> u32 {
    let socket_id = match socket_id_from_process_fd(process, fd) {
        Ok(socket_id) => socket_id,
        Err(errno) => return abi::errno(errno),
    };

    match query(socket_id) {
        Ok((ip, port)) => match write_sockaddr_in(process, addr_ptr, addrlen_ptr, ip, port) {
            Ok(()) => 0,
            Err(errno) => abi::errno(errno),
        },
        Err(error) => network_errno(error),
    }
}

fn socket_setsockopt_for_process(
    process: &Process,
    fd: u32,
    level: u32,
    optname: u32,
    optval: u32,
    optlen: u32,
) -> u32 {
    if optlen != 0 && optval == 0 {
        return abi::errno(abi::EFAULT);
    }
    let socket_id = match socket_id_from_process_fd(process, fd) {
        Ok(socket_id) => socket_id,
        Err(errno) => return abi::errno(errno),
    };

    match net::socket_setsockopt(socket_id, level, optname, optlen) {
        Ok(()) => 0,
        Err(error) => network_errno(error),
    }
}

fn read_sockaddr_in(process: &Process, addr_ptr: u32, addr_len: u32) -> Result<SockAddrIn, i32> {
    if addr_ptr == 0 {
        return Err(abi::EFAULT);
    }
    if addr_len < core::mem::size_of::<SockAddrIn>() as u32 {
        return Err(abi::EINVAL);
    }

    let mut addr = SockAddrIn::default();
    user::copy_from_user(
        &process.page_directory,
        addr_ptr,
        &mut addr as *mut SockAddrIn as *mut u8,
        core::mem::size_of::<SockAddrIn>() as u32,
    )
    .map_err(|_| abi::EFAULT)?;

    if addr.sin_family as u32 != net::AF_INET {
        return Err(abi::EINVAL);
    }

    Ok(addr)
}

fn sockaddr_ip_port(addr: SockAddrIn) -> ([u8; 4], u16) {
    (
        [
            ((addr.sin_addr >> 24) & 0xff) as u8,
            ((addr.sin_addr >> 16) & 0xff) as u8,
            ((addr.sin_addr >> 8) & 0xff) as u8,
            (addr.sin_addr & 0xff) as u8,
        ],
        u16::from_be(addr.sin_port),
    )
}

fn write_sockaddr_in(
    process: &Process,
    addr_ptr: u32,
    addrlen_ptr: u32,
    ip: [u8; 4],
    port: u16,
) -> Result<(), i32> {
    if addr_ptr == 0 || addrlen_ptr == 0 {
        return Err(abi::EFAULT);
    }

    let sockaddr_len = core::mem::size_of::<SockAddrIn>() as u32;
    let mut provided_len = 0_u32;
    user::copy_from_user(
        &process.page_directory,
        addrlen_ptr,
        &mut provided_len as *mut u32 as *mut u8,
        core::mem::size_of::<u32>() as u32,
    )
    .map_err(|_| abi::EFAULT)?;
    if provided_len < sockaddr_len {
        return Err(abi::EINVAL);
    }

    let addr = SockAddrIn {
        sin_family: net::AF_INET as u16,
        sin_port: port.to_be(),
        sin_addr: ((ip[0] as u32) << 24)
            | ((ip[1] as u32) << 16)
            | ((ip[2] as u32) << 8)
            | ip[3] as u32,
        sin_zero: [0; 8],
    };

    user::write_value(&process.page_directory, addr_ptr, &addr).map_err(|_| abi::EFAULT)?;
    user::write_value(&process.page_directory, addrlen_ptr, &sockaddr_len).map_err(|_| abi::EFAULT)
}

fn read_recvfrom_wait_args() -> Result<RecvFromArgs, i32> {
    KERNEL.with_task_manager(|tm| {
        let Some(current_task) = tm.get_current() else {
            return Err(abi::ESRCH);
        };

        let socket_id = current_task.read().get_stack_item(0);
        let buf_ptr = current_task.read().get_stack_item(1);
        let len = current_task.read().get_stack_item(2);
        let _flags = current_task.read().get_stack_item(3);
        let src_ptr = current_task.read().get_stack_item(4);
        let addrlen_ptr = current_task.read().get_stack_item(5);
        let _timeout_ticks = current_task.read().get_stack_item(6);

        if buf_ptr == 0 {
            return Err(abi::EFAULT);
        }
        if len == 0 {
            return Err(abi::EINVAL);
        }

        Ok(RecvFromArgs {
            socket_id,
            buf_ptr,
            len,
            src_ptr,
            addrlen_ptr,
        })
    })
}

fn write_recvfrom_result(args: RecvFromArgs, packet: net::SocketPacket) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return abi::errno(abi::ESRCH);
        };

        let process = current_task.read().process.clone();
        write_recvfrom_result_for_process(&process, args, packet)
    })
}

fn write_recvfrom_result_for_process(
    process: &Process,
    args: RecvFromArgs,
    packet: net::SocketPacket,
) -> u32 {
    let read_len = packet.data.len() as u32;
    if user::copy_to_user(
        &process.page_directory,
        args.buf_ptr,
        packet.data.as_ptr(),
        read_len,
    )
    .is_err()
    {
        return abi::errno(abi::EFAULT);
    }

    if args.src_ptr != 0 {
        if args.addrlen_ptr == 0 {
            return abi::errno(abi::EFAULT);
        }

        let mut provided_len = 0_u32;
        if user::copy_from_user(
            &process.page_directory,
            args.addrlen_ptr,
            &mut provided_len as *mut u32 as *mut u8,
            core::mem::size_of::<u32>() as u32,
        )
        .is_err()
        {
            return abi::errno(abi::EFAULT);
        }

        let sockaddr_len = core::mem::size_of::<SockAddrIn>() as u32;
        if provided_len < sockaddr_len {
            return abi::errno(abi::EINVAL);
        }

        let src = SockAddrIn {
            sin_family: net::AF_INET as u16,
            sin_port: packet.src_port.to_be(),
            sin_addr: ((packet.src_ip[0] as u32) << 24)
                | ((packet.src_ip[1] as u32) << 16)
                | ((packet.src_ip[2] as u32) << 8)
                | packet.src_ip[3] as u32,
            sin_zero: [0; 8],
        };

        if user::write_value(&process.page_directory, args.src_ptr, &src).is_err() {
            return abi::errno(abi::EFAULT);
        }

        let actual_len = sockaddr_len;
        if user::write_value(&process.page_directory, args.addrlen_ptr, &actual_len).is_err() {
            return abi::errno(abi::EFAULT);
        }
    }

    read_len
}

fn network_errno(error: net::NetworkError) -> u32 {
    let code = match error {
        net::NetworkError::NoInterface => abi::ENODEV,
        net::NetworkError::DeviceError => abi::EIO,
        net::NetworkError::PacketTooLarge => abi::EMSGSIZE,
        net::NetworkError::NotConfigured => abi::ENETDOWN,
        net::NetworkError::InvalidInput => abi::EINVAL,
        net::NetworkError::Unsupported => abi::ENOTSUP,
        net::NetworkError::BadSocket => abi::EBADF,
        net::NetworkError::WouldBlock => abi::EAGAIN,
        net::NetworkError::NotConnected => abi::ENOTCONN,
    };
    abi::errno(code)
}
