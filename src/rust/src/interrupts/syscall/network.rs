use crate::{
    interrupts::InterruptFrame,
    kernel::KERNEL,
    net,
    schedule::process::{ProcessDescriptor, SocketHandle},
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

pub fn syscall_network_info(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return abi::error();
        };

        let ptr = current_task.read().get_stack_item(0);
        if ptr == 0 {
            return abi::error();
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
            return abi::error();
        }

        0
    })
}

pub fn syscall_network_dhcp_discover(_frame: &InterruptFrame) -> u32 {
    match net::send_dhcp_discover() {
        Ok(()) => 0,
        Err(error) => {
            serial_println!("net: dhcp discover failed: {:?}", error);
            abi::error()
        }
    }
}

pub fn syscall_network_ping_gateway(_frame: &InterruptFrame) -> u32 {
    match net::ping_gateway() {
        Ok(()) => 0,
        Err(error) => {
            serial_println!("net: ping gateway failed: {:?}", error);
            abi::error()
        }
    }
}

pub fn syscall_network_ping_ipv4(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return abi::error();
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
                abi::error()
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

pub fn syscall_network_socket(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return abi::error();
        };

        let (process, domain, socket_type, protocol) = {
            let task = current_task.read();
            (
                task.process.clone(),
                task.get_stack_item(0),
                task.get_stack_item(1),
                task.get_stack_item(2),
            )
        };

        match net::socket_open(domain, socket_type, protocol) {
            Ok(socket_id) => match process.insert_fd(ProcessDescriptor::Socket(Arc::new(
                Mutex::new(SocketHandle::new(socket_id)),
            ))) {
                Ok(fd) => fd as u32,
                Err(error) => {
                    let _ = net::socket_close(socket_id);
                    serial_println!("net: socket fd allocation failed: {:?}", error);
                    abi::error()
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
                abi::error()
            }
        }
    })
}

pub fn syscall_network_sendto(_frame: &InterruptFrame) -> u32 {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return abi::error();
        };

        let (process, fd, _buf, len, _flags, dest_ptr, dest_len) = {
            let task = current_task.read();
            (
                task.process.clone(),
                task.get_stack_item(0),
                task.get_stack_item(1),
                task.get_stack_item(2),
                task.get_stack_item(3),
                task.get_stack_item(4),
                task.get_stack_item(5),
            )
        };
        if dest_ptr == 0 || dest_len < core::mem::size_of::<SockAddrIn>() as u32 {
            return abi::error();
        }

        let mut dest = SockAddrIn::default();
        if user::copy_from_user(
            &process.page_directory,
            dest_ptr,
            &mut dest as *mut SockAddrIn as *mut u8,
            core::mem::size_of::<SockAddrIn>() as u32,
        )
        .is_err()
        {
            return abi::error();
        }

        if dest.sin_family as u32 != net::AF_INET {
            return abi::error();
        }

        let target_ip = [
            ((dest.sin_addr >> 24) & 0xff) as u8,
            ((dest.sin_addr >> 16) & 0xff) as u8,
            ((dest.sin_addr >> 8) & 0xff) as u8,
            (dest.sin_addr & 0xff) as u8,
        ];
        let target_port = u16::from_be(dest.sin_port);

        let Some(ProcessDescriptor::Socket(socket)) = process.get_fd(fd as i32) else {
            return abi::error();
        };
        let socket_id = socket.lock().id();

        match net::socket_send_to(socket_id, len as usize, target_ip, target_port) {
            Ok(sent) => sent as u32,
            Err(error) => {
                serial_println!("net: sendto({}) failed: {:?}", fd, error);
                abi::error()
            }
        }
    })
}

pub fn syscall_network_recvfrom(_frame: &InterruptFrame) -> u32 {
    let Some(args) = read_recvfrom_args(false) else {
        return abi::error();
    };
    let Some(socket_id) = socket_id_from_fd(args.socket_id) else {
        return abi::error();
    };

    let packet = match net::socket_recv_from(socket_id, args.len as usize) {
        Ok(packet) => packet,
        Err(net::NetworkError::WouldBlock) => return abi::error(),
        Err(error) => {
            serial_println!("net: recvfrom({}) failed: {:?}", args.socket_id, error);
            return abi::error();
        }
    };

    write_recvfrom_result(args, packet)
}

pub fn syscall_network_recvfrom_wait(_frame: &InterruptFrame) -> u32 {
    let Some(args) = read_recvfrom_args(true) else {
        return abi::error();
    };
    let Some(socket_id) = socket_id_from_fd(args.socket_id) else {
        return abi::error();
    };

    let timeout_ticks = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        Some(current_task.read().get_stack_item(6) as u64)
    });

    let Some(timeout_ticks) = timeout_ticks else {
        return abi::error();
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
                return abi::error();
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
            return abi::error();
        };

        let task = current_task.read();
        let name_ptr = task.get_stack_item(0);
        if name_ptr == 0 {
            return abi::error();
        }

        let Some(name) = user::read_c_string(&task, name_ptr, 256) else {
            return abi::error();
        };

        match request(name.as_str()) {
            Ok(()) => 0,
            Err(error) => {
                serial_println!("net: {} {} failed: {:?}", label, name, error);
                abi::error()
            }
        }
    })
}

fn socket_id_from_fd(fd: u32) -> Option<u32> {
    KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let process = current_task.read().process.clone();
        match process.get_fd(fd as i32) {
            Some(ProcessDescriptor::Socket(socket)) => Some(socket.lock().id()),
            _ => None,
        }
    })
}

fn read_recvfrom_args(require_timeout_arg: bool) -> Option<RecvFromArgs> {
    KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;

        let socket_id = current_task.read().get_stack_item(0);
        let buf_ptr = current_task.read().get_stack_item(1);
        let len = current_task.read().get_stack_item(2);
        let _flags = current_task.read().get_stack_item(3);
        let src_ptr = current_task.read().get_stack_item(4);
        let addrlen_ptr = current_task.read().get_stack_item(5);

        if require_timeout_arg {
            let _timeout_ticks = current_task.read().get_stack_item(6);
        }

        if buf_ptr == 0 || len == 0 {
            return None;
        }

        Some(RecvFromArgs {
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
            return abi::error();
        };

        let read_len = packet.data.len() as u32;
        if user::copy_to_user(
            &current_task.read().process.page_directory,
            args.buf_ptr,
            packet.data.as_ptr(),
            read_len,
        )
        .is_err()
        {
            return abi::error();
        }

        if args.src_ptr != 0 {
            if args.addrlen_ptr == 0 {
                return abi::error();
            }

            let Some(provided_len) = user::read_u32(&current_task.read(), args.addrlen_ptr) else {
                return abi::error();
            };

            let sockaddr_len = core::mem::size_of::<SockAddrIn>() as u32;
            if provided_len < sockaddr_len {
                return abi::error();
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

            if user::write_value(
                &current_task.read().process.page_directory,
                args.src_ptr,
                &src,
            )
            .is_err()
            {
                return abi::error();
            }

            let actual_len = sockaddr_len;
            if user::write_value(
                &current_task.read().process.page_directory,
                args.addrlen_ptr,
                &actual_len,
            )
            .is_err()
            {
                return abi::error();
            }
        }

        read_len
    })
}
