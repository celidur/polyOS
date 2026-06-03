#![allow(dead_code)]

mod arp;
mod dhcp;
mod dns;
mod icmp;
mod packet;
mod udp;

use alloc::{collections::VecDeque, string::String, string::ToString, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;

use self::packet::{ETHERTYPE_ARP, ETHERTYPE_IPV4, IP_PROTOCOL_ICMP, ethertype, ipv4_packet};

pub type DeviceId = usize;
pub type InterfaceId = usize;

pub trait NetworkDevice: Sync {
    fn read(&self) -> Option<Vec<u8>>;
    fn write(&self, frame: &[u8]) -> Result<(), NetworkError>;
}

pub const AF_INET: u32 = 2;
pub const SOCK_DGRAM: u32 = 2;
pub const SOCK_RAW: u32 = 3;
pub const IPPROTO_ICMP: u32 = 1;
pub const IPPROTO_UDP: u32 = 17;
const SOCKET_RECV_QUEUE_LIMIT: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkError {
    NoInterface,
    DeviceError,
    PacketTooLarge,
    NotConfigured,
    InvalidInput,
    Unsupported,
    BadSocket,
    WouldBlock,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DhcpState {
    Init = 0,
    Selecting = 1,
    Requesting = 2,
    Bound = 3,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Ipv4Config {
    pub address: [u8; 4],
    pub subnet_mask: [u8; 4],
    pub router: [u8; 4],
    pub dns: [u8; 4],
    pub lease_time: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct NetworkInfo {
    pub mac: [u8; 6],
    pub packets_rx: u64,
    pub packets_tx: u64,
    pub dhcp_state: DhcpState,
    pub ipv4: Ipv4Config,
    pub arp_entries: u32,
    pub ping_tx: u32,
    pub ping_rx: u32,
    pub dns_tx: u32,
    pub dns_rx: u32,
}

struct NetworkInterface {
    name: &'static str,
    device_id: DeviceId,
    mac: [u8; 6],
    device: &'static dyn NetworkDevice,
    packets_rx: u64,
    packets_tx: u64,
    dhcp: DhcpClient,
    arp_cache: Vec<ArpEntry>,
    pending_ping: Option<PingRequest>,
    pending_dns: Option<DnsRequest>,
    ping_sequence: u16,
    dns_sequence: u16,
    ping_tx: u32,
    ping_rx: u32,
    dns_tx: u32,
    dns_rx: u32,
}

struct NetworkSocket {
    id: u32,
    domain: u32,
    socket_type: u32,
    protocol: u32,
    recv_queue: VecDeque<SocketPacket>,
}

pub struct SocketPacket {
    pub src_ip: [u8; 4],
    pub src_port: u16,
    pub data: Vec<u8>,
}

#[derive(Clone, Copy)]
struct DhcpClient {
    state: DhcpState,
    xid: u32,
    offered_ip: [u8; 4],
    server_id: [u8; 4],
    config: Ipv4Config,
}

impl Default for DhcpClient {
    fn default() -> Self {
        Self {
            state: DhcpState::Init,
            xid: dhcp::CLIENT_XID,
            offered_ip: [0; 4],
            server_id: [0; 4],
            config: Ipv4Config::default(),
        }
    }
}

#[derive(Clone, Copy)]
struct ArpEntry {
    ip: [u8; 4],
    mac: [u8; 6],
}

struct PingRequest {
    target_ip: [u8; 4],
    next_hop_ip: [u8; 4],
    target_name: Option<String>,
    socket_id: Option<u32>,
    id: u16,
    sequence: u16,
    sent_at_tsc: u64,
}

struct DnsRequest {
    name: String,
    server_ip: [u8; 4],
    next_hop_ip: [u8; 4],
    id: u16,
    source_port: u16,
    payload: Vec<u8>,
    action: DnsAction,
}

#[derive(Clone, Copy)]
enum DnsAction {
    Resolve,
    Ping,
}

struct NetworkStack {
    interfaces: Vec<NetworkInterface>,
    sockets: Vec<NetworkSocket>,
    next_socket_id: u32,
}

impl Default for NetworkStack {
    fn default() -> Self {
        Self {
            interfaces: Vec::new(),
            sockets: Vec::new(),
            next_socket_id: 3,
        }
    }
}

lazy_static! {
    static ref STACK: Mutex<NetworkStack> = Mutex::new(NetworkStack::default());
}

pub fn register_interface(
    name: &'static str,
    device_id: DeviceId,
    mac: [u8; 6],
    device: &'static dyn NetworkDevice,
) -> InterfaceId {
    let mut stack = STACK.lock();
    let id = stack.interfaces.len();
    stack.interfaces.push(NetworkInterface {
        name,
        device_id,
        mac,
        device,
        packets_rx: 0,
        packets_tx: 0,
        dhcp: DhcpClient::default(),
        arp_cache: Vec::new(),
        pending_ping: None,
        pending_dns: None,
        ping_sequence: 0,
        dns_sequence: 0,
        ping_tx: 0,
        ping_rx: 0,
        dns_tx: 0,
        dns_rx: 0,
    });

    serial_println!(
        "net: registered {} as net{} mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        name,
        id,
        mac[0],
        mac[1],
        mac[2],
        mac[3],
        mac[4],
        mac[5]
    );

    id
}

pub fn info() -> Option<NetworkInfo> {
    let stack = STACK.lock();
    stack.interfaces.first().map(|interface| NetworkInfo {
        mac: interface.mac,
        packets_rx: interface.packets_rx,
        packets_tx: interface.packets_tx,
        dhcp_state: interface.dhcp.state,
        ipv4: interface.dhcp.config,
        arp_entries: interface.arp_cache.len() as u32,
        ping_tx: interface.ping_tx,
        ping_rx: interface.ping_rx,
        dns_tx: interface.dns_tx,
        dns_rx: interface.dns_rx,
    })
}

pub fn receive(interface_id: InterfaceId, frame: &[u8]) -> Option<Vec<u8>> {
    let mut stack = STACK.lock();
    let socket_packet = {
        let interface = stack.interfaces.get_mut(interface_id)?;
        interface.packets_rx += 1;

        log_packet_summary(interface_id, interface, frame);

        if let Some(response) = handle_arp_frame(interface_id, interface, frame) {
            return Some(response);
        }

        if let Some(response) = handle_dhcp_frame(interface_id, interface, frame) {
            return Some(response);
        }

        if let Some(response) = handle_dns_frame(interface_id, interface, frame) {
            return Some(response);
        }

        handle_icmp_frame(interface_id, interface, frame)
    };

    if let Some((socket_id, packet)) = socket_packet {
        enqueue_socket_packet(&mut stack, socket_id, packet);
    }

    None
}

pub fn notify_tx(interface_id: InterfaceId) {
    if let Some(interface) = STACK.lock().interfaces.get_mut(interface_id) {
        interface.packets_tx += 1;
    }
}

pub fn socket_open(domain: u32, socket_type: u32, protocol: u32) -> Result<u32, NetworkError> {
    if domain != AF_INET {
        return Err(NetworkError::Unsupported);
    }

    if socket_type != SOCK_RAW || protocol != IPPROTO_ICMP {
        return Err(NetworkError::Unsupported);
    }

    let mut stack = STACK.lock();
    let id = stack.next_socket_id;
    stack.next_socket_id = stack.next_socket_id.wrapping_add(1).max(3);
    stack.sockets.push(NetworkSocket {
        id,
        domain,
        socket_type,
        protocol,
        recv_queue: VecDeque::new(),
    });
    Ok(id)
}

pub fn socket_close(socket_id: u32) -> Result<(), NetworkError> {
    let mut stack = STACK.lock();
    let Some(index) = stack
        .sockets
        .iter()
        .position(|socket| socket.id == socket_id)
    else {
        return Err(NetworkError::BadSocket);
    };

    stack.sockets.remove(index);
    Ok(())
}

pub fn socket_send_to(
    socket_id: u32,
    len: usize,
    dest_ip: [u8; 4],
    _dest_port: u16,
) -> Result<usize, NetworkError> {
    let (socket_type, protocol) = {
        let stack = STACK.lock();
        let socket = stack
            .sockets
            .iter()
            .find(|socket| socket.id == socket_id)
            .ok_or(NetworkError::BadSocket)?;
        (socket.socket_type, socket.protocol)
    };

    match (socket_type, protocol) {
        (SOCK_RAW, IPPROTO_ICMP) => {
            ping_ipv4_from_socket(socket_id, dest_ip)?;
            Ok(len)
        }
        _ => Err(NetworkError::Unsupported),
    }
}

pub fn socket_recv_from(socket_id: u32, max_len: usize) -> Result<SocketPacket, NetworkError> {
    let mut stack = STACK.lock();
    let socket = stack
        .sockets
        .iter_mut()
        .find(|socket| socket.id == socket_id)
        .ok_or(NetworkError::BadSocket)?;

    let mut packet = if let Some(packet) = socket.recv_queue.pop_front() {
        packet
    } else {
        return Err(NetworkError::WouldBlock);
    };

    if packet.data.len() > max_len {
        packet.data.truncate(max_len);
    }

    Ok(packet)
}

pub fn send_dhcp_discover() -> Result<(), NetworkError> {
    let (interface_id, device, frame) = {
        let mut stack = STACK.lock();
        let interface_id = 0;
        let interface = stack
            .interfaces
            .get_mut(interface_id)
            .ok_or(NetworkError::NoInterface)?;

        interface.dhcp = DhcpClient {
            state: DhcpState::Selecting,
            xid: dhcp::CLIENT_XID,
            ..DhcpClient::default()
        };

        (
            interface_id,
            interface.device,
            dhcp::build_discover(interface.mac, interface.dhcp.xid),
        )
    };

    device.write(&frame)?;
    notify_tx(interface_id);

    serial_println!(
        "net{}: tx dhcp discover xid=0x{:08x} len={}",
        interface_id,
        dhcp::CLIENT_XID,
        frame.len()
    );

    Ok(())
}

pub fn ping_gateway() -> Result<(), NetworkError> {
    let router = {
        let stack = STACK.lock();
        let interface = stack.interfaces.first().ok_or(NetworkError::NoInterface)?;
        if interface.dhcp.state != DhcpState::Bound || interface.dhcp.config.router == [0; 4] {
            return Err(NetworkError::NotConfigured);
        }
        interface.dhcp.config.router
    };

    ping_ipv4(router)
}

pub fn ping_ipv4(target_ip: [u8; 4]) -> Result<(), NetworkError> {
    send_ping_ipv4(target_ip, None, None)
}

fn ping_ipv4_from_socket(socket_id: u32, target_ip: [u8; 4]) -> Result<(), NetworkError> {
    send_ping_ipv4(target_ip, None, Some(socket_id))
}

fn send_ping_ipv4(
    target_ip: [u8; 4],
    target_name: Option<String>,
    socket_id: Option<u32>,
) -> Result<(), NetworkError> {
    let (interface_id, device, frame) = {
        let mut stack = STACK.lock();
        let interface_id = 0;
        let interface = stack
            .interfaces
            .get_mut(interface_id)
            .ok_or(NetworkError::NoInterface)?;

        let frame = prepare_ping_frame(interface_id, interface, target_ip, target_name, socket_id)?;
        (
            interface_id,
            interface.device,
            frame,
        )
    };

    device.write(&frame)?;
    notify_tx(interface_id);

    Ok(())
}

pub fn send_dns_query(name: &str) -> Result<(), NetworkError> {
    send_dns_request(name, DnsAction::Resolve)
}

pub fn ping_name(name: &str) -> Result<(), NetworkError> {
    send_dns_request(name, DnsAction::Ping)
}

fn send_dns_request(name: &str, action: DnsAction) -> Result<(), NetworkError> {
    let name = name.trim().trim_end_matches('.');
    if name.is_empty() {
        return Err(NetworkError::InvalidInput);
    }

    let (interface_id, device, frame, name, server_ip, next_hop_ip, arp_needed) = {
        let mut stack = STACK.lock();
        let interface_id = 0;
        let interface = stack
            .interfaces
            .get_mut(interface_id)
            .ok_or(NetworkError::NoInterface)?;

        if interface.dhcp.state != DhcpState::Bound
            || interface.dhcp.config.address == [0; 4]
            || interface.dhcp.config.dns == [0; 4]
        {
            return Err(NetworkError::NotConfigured);
        }

        interface.dns_sequence = interface.dns_sequence.wrapping_add(1);
        let id = 0x4400 | (interface.dns_sequence & 0x00FF);
        let source_port = 49152 + (interface.dns_sequence % 1024);
        let server_ip = interface.dhcp.config.dns;
        let next_hop_ip = route_next_hop(interface.dhcp.config, server_ip)?;
        let payload = dns::build_query(name, id).ok_or(NetworkError::InvalidInput)?;
        let request_name = name.to_string();

        interface.pending_dns = Some(DnsRequest {
            name: request_name.clone(),
            server_ip,
            next_hop_ip,
            id,
            source_port,
            payload: payload.clone(),
            action,
        });

        if let Some(next_hop_mac) = find_arp(interface, next_hop_ip) {
            interface.dns_tx += 1;
            (
                interface_id,
                interface.device,
                udp::build_ipv4_frame(
                    interface.mac,
                    next_hop_mac,
                    interface.dhcp.config.address,
                    server_ip,
                    source_port,
                    dns::DNS_PORT,
                    id,
                    &payload,
                ),
                request_name,
                server_ip,
                next_hop_ip,
                false,
            )
        } else {
            (
                interface_id,
                interface.device,
                arp::build_request(interface.mac, interface.dhcp.config.address, next_hop_ip),
                request_name,
                server_ip,
                next_hop_ip,
                true,
            )
        }
    };

    device.write(&frame)?;
    notify_tx(interface_id);

    if arp_needed {
        serial_println!(
            "net{}: tx arp who-has {}.{}.{}.{} for {} {}",
            interface_id,
            next_hop_ip[0],
            next_hop_ip[1],
            next_hop_ip[2],
            next_hop_ip[3],
            dns_action_name(action),
            name
        );
    } else {
        serial_println!(
            "net{}: tx dns query {} server={}.{}.{}.{} action={}",
            interface_id,
            name,
            server_ip[0],
            server_ip[1],
            server_ip[2],
            server_ip[3],
            dns_action_name(action)
        );
    }

    Ok(())
}

fn prepare_ping_frame(
    interface_id: InterfaceId,
    interface: &mut NetworkInterface,
    target_ip: [u8; 4],
    target_name: Option<String>,
    socket_id: Option<u32>,
) -> Result<Vec<u8>, NetworkError> {
    if interface.dhcp.state != DhcpState::Bound || interface.dhcp.config.address == [0; 4] {
        return Err(NetworkError::NotConfigured);
    }

    let next_hop_ip = route_next_hop(interface.dhcp.config, target_ip)?;
    let sequence = interface.ping_sequence;
    interface.ping_sequence = interface.ping_sequence.wrapping_add(1);
    let request = PingRequest {
        target_ip,
        next_hop_ip,
        target_name,
        socket_id,
        id: 0x504F,
        sequence,
        sent_at_tsc: read_tsc(),
    };
    interface.pending_ping = Some(request);
    let pending = interface.pending_ping.as_ref().unwrap();
    print_ping_start(pending);
    let id = pending.id;
    let sequence = pending.sequence;

    if let Some(next_hop_mac) = find_arp(interface, next_hop_ip) {
        interface.ping_tx += 1;
        serial_println!(
            "net{}: tx icmp echo {}.{}.{}.{}",
            interface_id,
            target_ip[0],
            target_ip[1],
            target_ip[2],
            target_ip[3]
        );

        Ok(icmp::build_echo_request(
            interface.mac,
            next_hop_mac,
            interface.dhcp.config.address,
            target_ip,
            id,
            sequence,
        ))
    } else {
        serial_println!(
            "net{}: tx arp who-has {}.{}.{}.{} for ping {}.{}.{}.{}",
            interface_id,
            next_hop_ip[0],
            next_hop_ip[1],
            next_hop_ip[2],
            next_hop_ip[3],
            target_ip[0],
            target_ip[1],
            target_ip[2],
            target_ip[3]
        );

        Ok(arp::build_request(
            interface.mac,
            interface.dhcp.config.address,
            next_hop_ip,
        ))
    }
}

fn handle_arp_frame(
    interface_id: InterfaceId,
    interface: &mut NetworkInterface,
    frame: &[u8],
) -> Option<Vec<u8>> {
    let packet = arp::parse(frame)?;
    if packet.operation != 2 || packet.target_ip != interface.dhcp.config.address {
        return None;
    }

    update_arp(interface, packet.sender_ip, packet.sender_mac);
    serial_println!(
        "net{}: rx arp reply {}.{}.{}.{} is {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        interface_id,
        packet.sender_ip[0],
        packet.sender_ip[1],
        packet.sender_ip[2],
        packet.sender_ip[3],
        packet.sender_mac[0],
        packet.sender_mac[1],
        packet.sender_mac[2],
        packet.sender_mac[3],
        packet.sender_mac[4],
        packet.sender_mac[5]
    );

    if let Some(pending) = interface.pending_ping.as_mut()
        && pending.next_hop_ip == packet.sender_ip
    {
        pending.sent_at_tsc = read_tsc();
        let target_ip = pending.target_ip;
        let id = pending.id;
        let sequence = pending.sequence;
        interface.ping_tx += 1;
        serial_println!(
            "net{}: tx icmp echo {}.{}.{}.{}",
            interface_id,
            target_ip[0],
            target_ip[1],
            target_ip[2],
            target_ip[3]
        );

        return Some(icmp::build_echo_request(
            interface.mac,
            packet.sender_mac,
            interface.dhcp.config.address,
            target_ip,
            id,
            sequence,
        ));
    }

    build_pending_dns_after_arp(interface_id, interface, packet.sender_ip, packet.sender_mac)
}

fn build_pending_dns_after_arp(
    interface_id: InterfaceId,
    interface: &mut NetworkInterface,
    sender_ip: [u8; 4],
    sender_mac: [u8; 6],
) -> Option<Vec<u8>> {
    let pending = interface.pending_dns.as_ref()?;
    if pending.next_hop_ip != sender_ip {
        return None;
    }

    let name = pending.name.clone();
    let server_ip = pending.server_ip;
    let source_port = pending.source_port;
    let id = pending.id;
    let payload = pending.payload.clone();
    let action = pending.action;

    interface.dns_tx += 1;
    serial_println!(
        "net{}: tx dns query {} server={}.{}.{}.{} action={}",
        interface_id,
        name,
        server_ip[0],
        server_ip[1],
        server_ip[2],
        server_ip[3],
        dns_action_name(action)
    );

    Some(udp::build_ipv4_frame(
        interface.mac,
        sender_mac,
        interface.dhcp.config.address,
        server_ip,
        source_port,
        dns::DNS_PORT,
        id,
        &payload,
    ))
}

fn dns_action_name(action: DnsAction) -> &'static str {
    match action {
        DnsAction::Resolve => "dns",
        DnsAction::Ping => "ping",
    }
}

fn handle_dhcp_frame(
    interface_id: InterfaceId,
    interface: &mut NetworkInterface,
    frame: &[u8],
) -> Option<Vec<u8>> {
    let packet = dhcp::parse(frame)?;
    if packet.xid != interface.dhcp.xid {
        return None;
    }

    match packet.message_type {
        2 => {
            interface.dhcp.state = DhcpState::Requesting;
            interface.dhcp.offered_ip = packet.yiaddr;
            interface.dhcp.server_id = packet.server_id;
            interface.dhcp.config = Ipv4Config {
                address: packet.yiaddr,
                subnet_mask: packet.subnet_mask,
                router: packet.router,
                dns: packet.dns,
                lease_time: packet.lease_time,
            };

            serial_println!(
                "net{}: rx dhcp offer ip={}.{}.{}.{} server={}.{}.{}.{}",
                interface_id,
                packet.yiaddr[0],
                packet.yiaddr[1],
                packet.yiaddr[2],
                packet.yiaddr[3],
                packet.server_id[0],
                packet.server_id[1],
                packet.server_id[2],
                packet.server_id[3]
            );

            let request =
                dhcp::build_request(interface.mac, packet.xid, packet.yiaddr, packet.server_id);
            serial_println!(
                "net{}: tx dhcp request ip={}.{}.{}.{}",
                interface_id,
                packet.yiaddr[0],
                packet.yiaddr[1],
                packet.yiaddr[2],
                packet.yiaddr[3]
            );
            Some(request)
        }
        5 => {
            interface.dhcp.state = DhcpState::Bound;
            interface.dhcp.config = Ipv4Config {
                address: packet.yiaddr,
                subnet_mask: packet.subnet_mask,
                router: packet.router,
                dns: packet.dns,
                lease_time: packet.lease_time,
            };

            serial_println!(
                "net{}: rx dhcp ack ip={}.{}.{}.{} router={}.{}.{}.{} dns={}.{}.{}.{}",
                interface_id,
                packet.yiaddr[0],
                packet.yiaddr[1],
                packet.yiaddr[2],
                packet.yiaddr[3],
                packet.router[0],
                packet.router[1],
                packet.router[2],
                packet.router[3],
                packet.dns[0],
                packet.dns[1],
                packet.dns[2],
                packet.dns[3]
            );
            None
        }
        6 => {
            interface.dhcp.state = DhcpState::Init;
            serial_println!("net{}: rx dhcp nak", interface_id);
            None
        }
        _ => {
            serial_println!(
                "net{}: rx dhcp {} ({})",
                interface_id,
                dhcp::message_name(packet.message_type),
                packet.message_type
            );
            None
        }
    }
}

fn handle_dns_frame(
    interface_id: InterfaceId,
    interface: &mut NetworkInterface,
    frame: &[u8],
) -> Option<Vec<u8>> {
    let Some(response) = dns::parse_response(frame) else {
        return None;
    };

    let Some(pending) = interface.pending_dns.as_ref() else {
        return None;
    };

    if response.id != pending.id
        || response.src_ip != pending.server_ip
        || response.dst_port != pending.source_port
    {
        return None;
    }

    let name = pending.name.clone();
    let action = pending.action;
    interface.dns_rx += 1;
    interface.pending_dns = None;

    if let Some(ip) = response.answer {
        serial_println!(
            "net{}: rx dns {} -> {}.{}.{}.{}",
            interface_id,
            name,
            ip[0],
            ip[1],
            ip[2],
            ip[3]
        );

        if let DnsAction::Ping = action {
            serial_println!(
                "net{}: dns resolved {}, starting ping {}.{}.{}.{}",
                interface_id,
                name,
                ip[0],
                ip[1],
                ip[2],
                ip[3]
            );

            return prepare_ping_frame(interface_id, interface, ip, Some(name.clone()), None)
                .map(Some)
                .unwrap_or_else(|error| {
                    serial_println!("net{}: ping {} failed: {:?}", interface_id, name, error);
                    None
                });
        }
    } else {
        serial_println!(
            "net{}: rx dns {} no A record rcode={}",
            interface_id,
            name,
            response.rcode
        );
    }

    None
}

fn handle_icmp_frame(
    interface_id: InterfaceId,
    interface: &mut NetworkInterface,
    frame: &[u8],
) -> Option<(u32, SocketPacket)> {
    let Some(reply) = icmp::parse_echo_reply(frame) else {
        return None;
    };

    let Some(pending) = interface.pending_ping.as_ref() else {
        return None;
    };

    if reply.id != pending.id
        || reply.sequence != pending.sequence
        || reply.src_ip != pending.target_ip
    {
        return None;
    }

    let sequence = pending.sequence;
    let sent_at_tsc = pending.sent_at_tsc;
    let socket_id = pending.socket_id;
    interface.ping_rx += 1;
    interface.pending_ping = None;
    print_ping_reply(&reply, sent_at_tsc);
    serial_println!(
        "net{}: rx icmp echo reply from {}.{}.{}.{} seq={}",
        interface_id,
        reply.src_ip[0],
        reply.src_ip[1],
        reply.src_ip[2],
        reply.src_ip[3],
        sequence
    );

    socket_id.and_then(|socket_id| {
        socket_packet_from_ipv4(frame, reply.src_ip, 0).map(|packet| (socket_id, packet))
    })
}

fn socket_packet_from_ipv4(frame: &[u8], src_ip: [u8; 4], src_port: u16) -> Option<SocketPacket> {
    let ipv4 = ipv4_packet(frame)?;
    let ip_offset = 14;
    let ip_len = ipv4.payload_offset + ipv4.payload_len - ip_offset;
    if frame.len() < ip_offset + ip_len {
        return None;
    }

    let mut data = Vec::new();
    data.extend_from_slice(&frame[ip_offset..ip_offset + ip_len]);
    Some(SocketPacket {
        src_ip,
        src_port,
        data,
    })
}

fn enqueue_socket_packet(stack: &mut NetworkStack, socket_id: u32, packet: SocketPacket) {
    let Some(socket) = stack
        .sockets
        .iter_mut()
        .find(|socket| socket.id == socket_id)
    else {
        return;
    };

    if socket.recv_queue.len() >= SOCKET_RECV_QUEUE_LIMIT {
        let _ = socket.recv_queue.pop_front();
    }

    socket.recv_queue.push_back(packet);
}

fn print_ping_start(request: &PingRequest) {
    let ip = request.target_ip;
    if let Some(name) = request.target_name.as_ref() {
        println!(
            "PING {} ({}.{}.{}.{}): {} data bytes",
            name,
            ip[0],
            ip[1],
            ip[2],
            ip[3],
            icmp::ECHO_PAYLOAD_BYTES
        );
        serial_println!(
            "PING {} ({}.{}.{}.{}): {} data bytes",
            name,
            ip[0],
            ip[1],
            ip[2],
            ip[3],
            icmp::ECHO_PAYLOAD_BYTES
        );
    } else {
        println!(
            "PING {}.{}.{}.{} ({}.{}.{}.{}): {} data bytes",
            ip[0],
            ip[1],
            ip[2],
            ip[3],
            ip[0],
            ip[1],
            ip[2],
            ip[3],
            icmp::ECHO_PAYLOAD_BYTES
        );
        serial_println!(
            "PING {}.{}.{}.{} ({}.{}.{}.{}): {} data bytes",
            ip[0],
            ip[1],
            ip[2],
            ip[3],
            ip[0],
            ip[1],
            ip[2],
            ip[3],
            icmp::ECHO_PAYLOAD_BYTES
        );
    }

    println!(
        "{} bytes to {}.{}.{}.{}: icmp_seq={}",
        icmp::ECHO_PACKET_BYTES,
        ip[0],
        ip[1],
        ip[2],
        ip[3],
        request.sequence
    );
    serial_println!(
        "{} bytes to {}.{}.{}.{}: icmp_seq={}",
        icmp::ECHO_PACKET_BYTES,
        ip[0],
        ip[1],
        ip[2],
        ip[3],
        request.sequence
    );
}

fn print_ping_reply(reply: &icmp::EchoReply, sent_at_tsc: u64) {
    let ip = reply.src_ip;
    let elapsed_us = elapsed_microseconds(sent_at_tsc, read_tsc());
    let elapsed_ms = elapsed_us / 1000;
    let elapsed_ms_fraction = elapsed_us % 1000;

    println!(
        "{} bytes from {}.{}.{}.{}: icmp_seq={} ttl={} time={}.{:03} ms",
        reply.bytes,
        ip[0],
        ip[1],
        ip[2],
        ip[3],
        reply.sequence,
        reply.ttl,
        elapsed_ms,
        elapsed_ms_fraction
    );
    serial_println!(
        "{} bytes from {}.{}.{}.{}: icmp_seq={} ttl={} time={}.{:03} ms",
        reply.bytes,
        ip[0],
        ip[1],
        ip[2],
        ip[3],
        reply.sequence,
        reply.ttl,
        elapsed_ms,
        elapsed_ms_fraction
    );
}

fn elapsed_microseconds(start: u64, end: u64) -> u64 {
    const TSC_CYCLES_PER_MICROSECOND: u64 = 3000;

    (end.saturating_sub(start) / TSC_CYCLES_PER_MICROSECOND).max(1)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn read_tsc() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }
    ((high as u64) << 32) | low as u64
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn read_tsc() -> u64 {
    0
}

fn log_packet_summary(interface_id: InterfaceId, interface: &NetworkInterface, frame: &[u8]) {
    let Some(kind) = ethertype(frame) else {
        return;
    };

    if kind == ETHERTYPE_ARP {
        return;
    }

    if dhcp::parse(frame).is_some() {
        return;
    }

    if dns::parse_response(frame).is_some() {
        return;
    }

    if kind == ETHERTYPE_IPV4
        && let Some(ipv4) = ipv4_packet(frame)
        && ipv4.protocol == IP_PROTOCOL_ICMP
    {
        return;
    }

    if interface.packets_rx <= 4 {
        let protocol = if kind == ETHERTYPE_IPV4 {
            ipv4_packet(frame)
                .map(|packet| packet.protocol)
                .unwrap_or(0)
        } else {
            0
        };
        serial_println!(
            "net{}: rx len={} ethertype=0x{:04x} proto={}",
            interface_id,
            frame.len(),
            kind,
            protocol
        );
    }
}

fn find_arp(interface: &NetworkInterface, ip: [u8; 4]) -> Option<[u8; 6]> {
    interface
        .arp_cache
        .iter()
        .find(|entry| entry.ip == ip)
        .map(|entry| entry.mac)
}

fn update_arp(interface: &mut NetworkInterface, ip: [u8; 4], mac: [u8; 6]) {
    if let Some(entry) = interface.arp_cache.iter_mut().find(|entry| entry.ip == ip) {
        entry.mac = mac;
        return;
    }

    interface.arp_cache.push(ArpEntry { ip, mac });
}

fn route_next_hop(config: Ipv4Config, target_ip: [u8; 4]) -> Result<[u8; 4], NetworkError> {
    if target_ip == [0; 4] {
        return Err(NetworkError::NotConfigured);
    }

    if same_subnet(config.address, target_ip, config.subnet_mask) {
        Ok(target_ip)
    } else if config.router != [0; 4] {
        Ok(config.router)
    } else {
        Err(NetworkError::NotConfigured)
    }
}

fn same_subnet(a: [u8; 4], b: [u8; 4], mask: [u8; 4]) -> bool {
    (a[0] & mask[0]) == (b[0] & mask[0])
        && (a[1] & mask[1]) == (b[1] & mask[1])
        && (a[2] & mask[2]) == (b[2] & mask[2])
        && (a[3] & mask[3]) == (b[3] & mask[3])
}
