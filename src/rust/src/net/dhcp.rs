use alloc::vec::Vec;

use super::packet::{
    BROADCAST_MAC, ETHERTYPE_IPV4, IP_PROTOCOL_UDP, UDP_PORT_DHCP_CLIENT, UDP_PORT_DHCP_SERVER,
    internet_checksum, ipv4_packet, is_dhcp_ports, read_be16, read_be32, read_ipv4, write_be16,
    write_be32, write_ethernet_header,
};

pub const CLIENT_XID: u32 = 0x504F4C59;

#[derive(Clone, Copy, Default)]
pub struct DhcpPacket {
    pub message_type: u8,
    pub xid: u32,
    pub yiaddr: [u8; 4],
    pub server_id: [u8; 4],
    pub subnet_mask: [u8; 4],
    pub router: [u8; 4],
    pub dns: [u8; 4],
    pub lease_time: u32,
}

pub fn parse(frame: &[u8]) -> Option<DhcpPacket> {
    let ipv4 = ipv4_packet(frame)?;
    if ipv4.protocol != IP_PROTOCOL_UDP || ipv4.payload_len < 8 {
        return None;
    }

    let udp_offset = ipv4.payload_offset;
    let src_port = read_be16(frame, udp_offset);
    let dst_port = read_be16(frame, udp_offset + 2);
    if !is_dhcp_ports(src_port, dst_port) {
        return None;
    }

    let dhcp_offset = udp_offset + 8;
    if frame.len() < dhcp_offset + 240 {
        return None;
    }

    let xid = read_be32(frame, dhcp_offset + 4);
    let yiaddr = read_ipv4(frame, dhcp_offset + 16);
    if read_be32(frame, dhcp_offset + 236) != 0x63825363 {
        return None;
    }

    let mut packet = DhcpPacket {
        xid,
        yiaddr,
        ..DhcpPacket::default()
    };

    let mut offset = dhcp_offset + 240;
    while offset < frame.len() {
        let code = frame[offset];
        match code {
            0 => offset += 1,
            255 => break,
            _ => {
                if offset + 1 >= frame.len() {
                    break;
                }
                let len = frame[offset + 1] as usize;
                let value = offset + 2;
                if value + len > frame.len() {
                    break;
                }

                match code {
                    1 if len == 4 => packet.subnet_mask = read_ipv4(frame, value),
                    3 if len >= 4 => packet.router = read_ipv4(frame, value),
                    6 if len >= 4 => packet.dns = read_ipv4(frame, value),
                    51 if len == 4 => packet.lease_time = read_be32(frame, value),
                    53 if len >= 1 => packet.message_type = frame[value],
                    54 if len == 4 => packet.server_id = read_ipv4(frame, value),
                    _ => {}
                }

                offset = value + len;
            }
        }
    }

    if packet.message_type == 0 {
        None
    } else {
        Some(packet)
    }
}

pub fn build_discover(mac: [u8; 6], xid: u32) -> Vec<u8> {
    build_client_frame(mac, xid, 1, None, None)
}

pub fn build_request(mac: [u8; 6], xid: u32, requested_ip: [u8; 4], server_id: [u8; 4]) -> Vec<u8> {
    build_client_frame(mac, xid, 3, Some(requested_ip), Some(server_id))
}

fn build_client_frame(
    mac: [u8; 6],
    xid: u32,
    message_type: u8,
    requested_ip: Option<[u8; 4]>,
    server_id: Option<[u8; 4]>,
) -> Vec<u8> {
    let mut options_len = 4 + 3 + 9 + 6 + 1;
    if requested_ip.is_some() {
        options_len += 6;
    }
    if server_id.is_some() {
        options_len += 6;
    }

    let dhcp_len = 236 + options_len;
    let udp_len = 8 + dhcp_len;
    let ip_len = 20 + udp_len;
    let frame_len = 14 + ip_len;
    let mut frame = vec![0; frame_len];

    write_ethernet_header(&mut frame, BROADCAST_MAC, mac, ETHERTYPE_IPV4);

    let ip = 14;
    frame[ip] = 0x45;
    write_be16(&mut frame, ip + 2, ip_len as u16);
    write_be16(&mut frame, ip + 4, 0x0001);
    write_be16(&mut frame, ip + 6, 0x0000);
    frame[ip + 8] = 64;
    frame[ip + 9] = IP_PROTOCOL_UDP;
    frame[ip + 16..ip + 20].fill(0xFF);
    let checksum = internet_checksum(&frame[ip..ip + 20]);
    write_be16(&mut frame, ip + 10, checksum);

    let udp = ip + 20;
    write_be16(&mut frame, udp, UDP_PORT_DHCP_CLIENT);
    write_be16(&mut frame, udp + 2, UDP_PORT_DHCP_SERVER);
    write_be16(&mut frame, udp + 4, udp_len as u16);
    write_be16(&mut frame, udp + 6, 0);

    let dhcp = udp + 8;
    frame[dhcp] = 1;
    frame[dhcp + 1] = 1;
    frame[dhcp + 2] = 6;
    write_be32(&mut frame, dhcp + 4, xid);
    write_be16(&mut frame, dhcp + 10, 0x8000);
    frame[dhcp + 28..dhcp + 34].copy_from_slice(&mac);

    let mut option = dhcp + 236;
    frame[option..option + 4].copy_from_slice(&[99, 130, 83, 99]);
    option += 4;
    frame[option..option + 3].copy_from_slice(&[53, 1, message_type]);
    option += 3;
    frame[option..option + 9]
        .copy_from_slice(&[61, 7, 1, mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]]);
    option += 9;

    if let Some(ip) = requested_ip {
        frame[option..option + 2].copy_from_slice(&[50, 4]);
        frame[option + 2..option + 6].copy_from_slice(&ip);
        option += 6;
    }

    if let Some(server) = server_id {
        frame[option..option + 2].copy_from_slice(&[54, 4]);
        frame[option + 2..option + 6].copy_from_slice(&server);
        option += 6;
    }

    frame[option..option + 6].copy_from_slice(&[55, 4, 1, 3, 6, 15]);
    option += 6;
    frame[option] = 255;

    frame
}

pub fn message_name(message_type: u8) -> &'static str {
    match message_type {
        1 => "discover",
        2 => "offer",
        3 => "request",
        4 => "decline",
        5 => "ack",
        6 => "nak",
        7 => "release",
        8 => "inform",
        _ => "unknown",
    }
}
