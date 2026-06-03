#![no_main]
#![no_std]

use core::ffi::c_void;

use bindings::{clear_screen, malloc, print_memory, reboot, shutdown};
use polyos_std::*;
use process::run;

#[polyos_std::main]
fn main() {
    let mut buffer = [0u8; 1024];
    println!("PolyOS v2.0.0");
    loop {
        print!("> ");
        buffer.fill(0);
        let len = polyos_std::stdio::terminal_readline(&mut buffer, true);
        let buffer = core::str::from_utf8(&buffer[..len]).unwrap();
        println!();
        if buffer.is_empty() {
            continue;
        }

        if let Some(target) = buffer.strip_prefix("ping ") {
            let target = target.trim();
            if target.is_empty() {
                println!("Usage: ping <a.b.c.d|name>");
            } else if let Some(ip) = parse_ipv4(target) {
                ping_ipv4(ip);
            } else {
                ping_name(target);
            }
            continue;
        }

        if let Some(name) = buffer.strip_prefix("dns ") {
            let name = name.trim();
            if name.is_empty() {
                println!("Usage: dns <name>");
            } else {
                send_dns_query(name);
            }
            continue;
        }

        match buffer {
            "memory" => unsafe {
                print_memory();
            },
            "exit" => break,
            "malloc" => {
                let ptr = unsafe { malloc(4096 * 4096) };
                println!("malloc: {:x}", ptr as u32);
            }
            "clear" => unsafe {
                clear_screen();
            },
            "winsize" => print_winsize(),
            "devtest" => devtest(),
            "net" => print_network_info(),
            "dhcp" => send_dhcp_discover(),
            "ping" => ping_gateway(),
            "reboot" => unsafe {
                reboot();
            },
            "shutdown" => unsafe {
                shutdown();
            },
            _ => {
                let status = run(buffer);
                if status < 0 || status == 127 {
                    println!("Command not found");
                } else {
                    println!("Process exited with status {}", status);
                }
            }
        }
    }
}

fn print_winsize() {
    let mut size = bindings::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let result = unsafe {
        bindings::ioctl(
            bindings::STDOUT_FILENO as i32,
            bindings::TIOCGWINSZ as core::ffi::c_ulong,
            &mut size as *mut bindings::winsize as core::ffi::c_ulong,
        )
    };

    if result < 0 {
        println!("winsize: ioctl failed");
        return;
    }

    println!("screen: {} cols x {} rows", size.ws_col, size.ws_row);
}

fn devtest() {
    test_device_write(b"/dev/serial\0", b"/dev/serial write ok\n", "/dev/serial");
    test_device_write(b"/dev/screen\0", b"/dev/screen write ok\n", "/dev/screen");
    test_device_write(b"/dev/null\0", b"/dev/null write ok\n", "/dev/null");
}

fn test_device_write(path: &'static [u8], message: &'static [u8], label: &str) {
    let fd = unsafe { bindings::open(path.as_ptr() as *const i8, bindings::O_WRONLY as i32, 0) };

    if fd < 0 {
        println!("{}: open failed", label);
        return;
    }

    let written = unsafe { bindings::write(fd, message.as_ptr() as *const c_void, message.len()) };
    unsafe {
        bindings::close(fd);
    }

    if written == message.len() as isize {
        println!("{}: ok", label);
    } else {
        println!("{}: write failed", label);
    }
}

fn print_network_info() {
    let mut info = bindings::network_info {
        present: 0,
        dhcp_state: 0,
        mac: [0; 6],
        _padding: [0; 2],
        ipv4: [0; 4],
        subnet_mask: [0; 4],
        router: [0; 4],
        dns: [0; 4],
        packets_rx: 0,
        packets_tx: 0,
        arp_entries: 0,
        ping_tx: 0,
        ping_rx: 0,
        dns_tx: 0,
        dns_rx: 0,
    };

    if unsafe { bindings::network_info(&mut info) } < 0 || info.present == 0 {
        println!("Network: no RTL8139 device");
        return;
    }

    println!(
        "Network: rtl8139 mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} rx={} tx={} dhcp={} arp={} ping={}/{} dns={}/{}",
        info.mac[0],
        info.mac[1],
        info.mac[2],
        info.mac[3],
        info.mac[4],
        info.mac[5],
        info.packets_rx,
        info.packets_tx,
        dhcp_state_name(info.dhcp_state),
        info.arp_entries,
        info.ping_rx,
        info.ping_tx,
        info.dns_rx,
        info.dns_tx
    );

    if info.ipv4 != [0; 4] {
        println!(
            "IPv4: {}.{}.{}.{}/{}.{}.{}.{} gateway={}.{}.{}.{} dns={}.{}.{}.{}",
            info.ipv4[0],
            info.ipv4[1],
            info.ipv4[2],
            info.ipv4[3],
            info.subnet_mask[0],
            info.subnet_mask[1],
            info.subnet_mask[2],
            info.subnet_mask[3],
            info.router[0],
            info.router[1],
            info.router[2],
            info.router[3],
            info.dns[0],
            info.dns[1],
            info.dns[2],
            info.dns[3]
        );
    }
}

fn send_dhcp_discover() {
    let res = unsafe { bindings::network_dhcp_discover() };
    if res < 0 {
        println!("DHCP discover failed");
        return;
    }

    println!("DHCP discover sent");
    print_network_info();
}

fn ping_gateway() {
    let res = unsafe { bindings::network_ping_gateway() };
    if res < 0 {
        println!("Ping failed: run dhcp first");
    }
}

fn ping_ipv4(ip: u32) {
    let socket = unsafe {
        bindings::socket(
            bindings::AF_INET as i32,
            bindings::SOCK_RAW as i32,
            bindings::IPPROTO_ICMP as i32,
        )
    };
    if socket < 0 {
        println!("Ping failed: socket");
        return;
    }

    let addr = bindings::sockaddr_in {
        sin_family: bindings::AF_INET as u16,
        sin_port: 0,
        sin_addr: bindings::in_addr { s_addr: ip },
        sin_zero: [0; 8],
    };

    let res = unsafe {
        bindings::sendto(
            socket,
            core::ptr::null::<core::ffi::c_void>(),
            64,
            0,
            &addr as *const bindings::sockaddr_in as *const bindings::sockaddr,
            core::mem::size_of::<bindings::sockaddr_in>() as u32,
        )
    };

    if res < 0 {
        unsafe {
            bindings::close(socket);
        }
        println!("Ping failed: run dhcp first");
        return;
    }

    if !recv_ping_reply(socket) {
        println!("Ping failed: timeout");
    }

    unsafe {
        bindings::close(socket);
    }
}

fn recv_ping_reply(socket: i32) -> bool {
    let mut packet = [0u8; 128];
    let mut src = bindings::sockaddr_in {
        sin_family: 0,
        sin_port: 0,
        sin_addr: bindings::in_addr { s_addr: 0 },
        sin_zero: [0; 8],
    };
    let mut src_len = core::mem::size_of::<bindings::sockaddr_in>() as bindings::socklen_t;

    let res = unsafe {
        bindings::recvfrom_wait(
            socket,
            packet.as_mut_ptr() as *mut core::ffi::c_void,
            packet.len(),
            0,
            &mut src as *mut bindings::sockaddr_in as *mut bindings::sockaddr,
            &mut src_len,
            5_000_000,
        )
    };

    res >= 0
}

fn ping_name(name: &str) {
    let res = unsafe { bindings::network_ping_name(name.as_ptr() as *const i8) };
    if res < 0 {
        println!("Ping failed: run dhcp first");
    }
}

fn send_dns_query(name: &str) {
    let res = unsafe { bindings::network_dns_query(name.as_ptr() as *const i8) };
    if res < 0 {
        println!("DNS query failed: run dhcp first");
        return;
    }

    println!("DNS query sent");
    print_network_info();
}

fn parse_ipv4(input: &str) -> Option<u32> {
    let mut octets = [0u8; 4];
    let mut count = 0;

    for part in input.split('.') {
        if count >= octets.len() {
            return None;
        }

        octets[count] = parse_ipv4_octet(part)?;
        count += 1;
    }

    if count != octets.len() {
        return None;
    }

    Some(
        ((octets[0] as u32) << 24)
            | ((octets[1] as u32) << 16)
            | ((octets[2] as u32) << 8)
            | octets[3] as u32,
    )
}

fn parse_ipv4_octet(input: &str) -> Option<u8> {
    if input.is_empty() {
        return None;
    }

    let mut value = 0u16;
    for byte in input.bytes() {
        if byte < b'0' || byte > b'9' {
            return None;
        }

        value = value * 10 + (byte - b'0') as u16;
        if value > 255 {
            return None;
        }
    }

    Some(value as u8)
}

fn dhcp_state_name(state: u32) -> &'static str {
    match state {
        0 => "init",
        1 => "selecting",
        2 => "requesting",
        3 => "bound",
        _ => "unknown",
    }
}
