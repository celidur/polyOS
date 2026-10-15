//! Transport socket state, addressing, packet routing and payload queues.
use super::*;

pub fn socket_open(domain: u32, socket_type: u32, protocol: u32) -> Result<u32, NetworkError> {
    if domain != AF_INET {
        return Err(NetworkError::Unsupported);
    }

    let protocol = if socket_type == SOCK_DGRAM && protocol == 0 {
        IPPROTO_UDP
    } else {
        protocol
    };

    if !((socket_type == SOCK_RAW && protocol == IPPROTO_ICMP)
        || (socket_type == SOCK_DGRAM && protocol == IPPROTO_UDP))
    {
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
        local_ip: [0; 4],
        local_port: 0,
        peer_ip: None,
        peer_port: 0,
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
    payload: &[u8],
    len: usize,
    dest_ip: [u8; 4],
    dest_port: u16,
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
        (SOCK_DGRAM, IPPROTO_UDP) if payload.len() == len => {
            socket_send_udp(socket_id, payload, dest_ip, dest_port)
        }
        (SOCK_DGRAM, IPPROTO_UDP) => Err(NetworkError::InvalidInput),
        _ => Err(NetworkError::Unsupported),
    }
}

fn socket_send_udp(
    socket_id: u32,
    payload: &[u8],
    dest_ip: [u8; 4],
    dest_port: u16,
) -> Result<usize, NetworkError> {
    if dest_port == 0 {
        return Err(NetworkError::InvalidInput);
    }
    if payload.len() > 1472 {
        return Err(NetworkError::PacketTooLarge);
    }

    let (interface_id, device, frame, arp_needed) = {
        let mut stack = STACK.lock();
        let socket_index = stack
            .sockets
            .iter()
            .position(|socket| socket.id == socket_id)
            .ok_or(NetworkError::BadSocket)?;

        if stack.sockets[socket_index].local_port == 0 {
            stack.sockets[socket_index].local_port = ephemeral_port(&stack.sockets)?;
        }
        let (local_ip, source_port) = {
            let socket = &mut stack.sockets[socket_index];
            if socket.socket_type != SOCK_DGRAM || socket.protocol != IPPROTO_UDP {
                return Err(NetworkError::Unsupported);
            }
            (socket.local_ip, socket.local_port)
        };

        let interface_id = 0;
        let interface = stack
            .interfaces
            .get_mut(interface_id)
            .ok_or(NetworkError::NoInterface)?;
        if interface.dhcp.state != DhcpState::Bound || interface.dhcp.config.address == [0; 4] {
            return Err(NetworkError::NotConfigured);
        }

        let source_ip = if local_ip == [0; 4] {
            interface.dhcp.config.address
        } else {
            local_ip
        };
        let next_hop_ip = route_next_hop(interface.dhcp.config, dest_ip)?;
        interface.dns_sequence = interface.dns_sequence.wrapping_add(1);
        let identification = 0x5500 | (interface.dns_sequence & 0x00ff);

        if let Some(next_hop_mac) = find_arp(interface, next_hop_ip) {
            (
                interface_id,
                interface.device,
                udp::build_ipv4_frame(
                    interface.mac,
                    next_hop_mac,
                    source_ip,
                    dest_ip,
                    source_port,
                    dest_port,
                    identification,
                    payload,
                ),
                false,
            )
        } else {
            (
                interface_id,
                interface.device,
                arp::build_request(interface.mac, source_ip, next_hop_ip),
                true,
            )
        }
    };

    device.write(&frame)?;
    notify_tx(interface_id);

    if arp_needed {
        serial_println!(
            "net{}: tx arp for udp {}.{}.{}.{}:{}",
            interface_id,
            dest_ip[0],
            dest_ip[1],
            dest_ip[2],
            dest_ip[3],
            dest_port
        );
        Err(NetworkError::WouldBlock)
    } else {
        serial_println!(
            "net{}: tx udp {}.{}.{}.{}:{} len={}",
            interface_id,
            dest_ip[0],
            dest_ip[1],
            dest_ip[2],
            dest_ip[3],
            dest_port,
            payload.len()
        );
        Ok(payload.len())
    }
}

pub fn socket_bind(socket_id: u32, local_ip: [u8; 4], local_port: u16) -> Result<(), NetworkError> {
    let mut stack = STACK.lock();
    let index = stack
        .sockets
        .iter()
        .position(|s| s.id == socket_id)
        .ok_or(NetworkError::BadSocket)?;
    if stack.sockets[index].socket_type == SOCK_DGRAM {
        let port = if local_port == 0 {
            ephemeral_port(&stack.sockets)?
        } else {
            local_port
        };
        if stack.sockets.iter().any(|s| {
            s.id != socket_id
                && s.socket_type == SOCK_DGRAM
                && s.local_port == port
                && (s.local_ip == [0; 4] || local_ip == [0; 4] || s.local_ip == local_ip)
        }) {
            return Err(NetworkError::AddressInUse);
        }
        if stack.sockets[index].local_port != 0 {
            return Err(NetworkError::InvalidInput);
        }
        stack.sockets[index].local_port = port;
    } else {
        stack.sockets[index].local_port = local_port;
    }
    stack.sockets[index].local_ip = local_ip;
    Ok(())
}

fn ephemeral_port(sockets: &[NetworkSocket]) -> Result<u16, NetworkError> {
    (49152..=65535)
        .find(|&p| !sockets.iter().any(|s| s.local_port == p))
        .ok_or(NetworkError::AddressInUse)
}

pub fn socket_connect(
    socket_id: u32,
    peer_ip: [u8; 4],
    peer_port: u16,
) -> Result<(), NetworkError> {
    let mut stack = STACK.lock();
    let socket = stack
        .sockets
        .iter_mut()
        .find(|socket| socket.id == socket_id)
        .ok_or(NetworkError::BadSocket)?;

    socket.peer_ip = Some(peer_ip);
    socket.peer_port = peer_port;
    Ok(())
}

pub fn socket_send(socket_id: u32, payload: &[u8], len: usize) -> Result<usize, NetworkError> {
    let (peer_ip, peer_port) = {
        let stack = STACK.lock();
        let socket = stack
            .sockets
            .iter()
            .find(|socket| socket.id == socket_id)
            .ok_or(NetworkError::BadSocket)?;
        (
            socket.peer_ip.ok_or(NetworkError::NotConnected)?,
            socket.peer_port,
        )
    };

    socket_send_to(socket_id, payload, len, peer_ip, peer_port)
}

pub fn socket_setsockopt(
    socket_id: u32,
    _level: u32,
    _optname: u32,
    _optlen: u32,
) -> Result<(), NetworkError> {
    let stack = STACK.lock();
    if stack.sockets.iter().any(|socket| socket.id == socket_id) {
        Ok(())
    } else {
        Err(NetworkError::BadSocket)
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

pub(super) fn socket_packet_from_ipv4(
    frame: &[u8],
    src_ip: [u8; 4],
    src_port: u16,
) -> Option<SocketPacket> {
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
        dst_ip: ipv4.dst,
        dst_port: 0,
        data,
    })
}

pub(super) fn socket_packet_from_udp_payload(frame: &[u8]) -> Option<SocketPacket> {
    let packet = udp::parse(frame)?;
    let mut data = Vec::new();
    data.extend_from_slice(
        &frame[packet.payload_offset..packet.payload_offset + packet.payload_len],
    );
    Some(SocketPacket {
        src_ip: packet.src_ip,
        src_port: packet.src_port,
        dst_ip: packet.dst_ip,
        dst_port: packet.dst_port,
        data,
    })
}

pub(super) fn udp_socket_id_for_packet(
    sockets: &[NetworkSocket],
    packet: &SocketPacket,
) -> Option<u32> {
    sockets
        .iter()
        .find(|socket| {
            socket.socket_type == SOCK_DGRAM
                && socket.protocol == IPPROTO_UDP
                && socket.local_port == packet.dst_port
                && (socket.local_ip == [0; 4] || socket.local_ip == packet.dst_ip)
                && socket
                    .peer_ip
                    .map(|peer_ip| peer_ip == packet.src_ip && socket.peer_port == packet.src_port)
                    .unwrap_or(true)
        })
        .map(|socket| socket.id)
}

pub(super) fn enqueue_socket_packet(
    stack: &mut NetworkStack,
    socket_id: u32,
    packet: SocketPacket,
) {
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

pub fn socket_local_addr(socket_id: u32) -> Result<([u8; 4], u16), NetworkError> {
    let stack = STACK.lock();
    let socket = stack
        .sockets
        .iter()
        .find(|socket| socket.id == socket_id)
        .ok_or(NetworkError::BadSocket)?;

    let ip = if socket.local_ip == [0; 4] {
        stack
            .interfaces
            .first()
            .map(|interface| interface.dhcp.config.address)
            .unwrap_or([0; 4])
    } else {
        socket.local_ip
    };

    Ok((ip, socket.local_port))
}

pub fn socket_peer_addr(socket_id: u32) -> Result<([u8; 4], u16), NetworkError> {
    let stack = STACK.lock();
    let socket = stack
        .sockets
        .iter()
        .find(|socket| socket.id == socket_id)
        .ok_or(NetworkError::BadSocket)?;

    Ok((
        socket.peer_ip.ok_or(NetworkError::NotConnected)?,
        socket.peer_port,
    ))
}
