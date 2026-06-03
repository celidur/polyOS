use alloc::vec::Vec;

use super::packet::{
    ETHERTYPE_IPV4, IP_PROTOCOL_UDP, internet_checksum, ipv4_packet, read_be16, write_be16,
    write_ethernet_header,
};

#[derive(Clone, Copy)]
pub struct UdpPacket {
    pub src_ip: [u8; 4],
    pub dst_ip: [u8; 4],
    pub src_port: u16,
    pub dst_port: u16,
    pub payload_offset: usize,
    pub payload_len: usize,
}

pub fn parse(frame: &[u8]) -> Option<UdpPacket> {
    let ipv4 = ipv4_packet(frame)?;
    if ipv4.protocol != IP_PROTOCOL_UDP || ipv4.payload_len < 8 {
        return None;
    }

    let udp = ipv4.payload_offset;
    let udp_len = read_be16(frame, udp + 4) as usize;
    if udp_len < 8 || udp_len > ipv4.payload_len {
        return None;
    }

    Some(UdpPacket {
        src_ip: ipv4.src,
        dst_ip: ipv4.dst,
        src_port: read_be16(frame, udp),
        dst_port: read_be16(frame, udp + 2),
        payload_offset: udp + 8,
        payload_len: udp_len - 8,
    })
}

pub fn build_ipv4_frame(
    src_mac: [u8; 6],
    dst_mac: [u8; 6],
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    identification: u16,
    payload: &[u8],
) -> Vec<u8> {
    let udp_len = 8 + payload.len();
    let ip_len = 20 + udp_len;
    let mut frame = vec![0; 14 + ip_len];

    write_ethernet_header(&mut frame, dst_mac, src_mac, ETHERTYPE_IPV4);

    let ip = 14;
    frame[ip] = 0x45;
    write_be16(&mut frame, ip + 2, ip_len as u16);
    write_be16(&mut frame, ip + 4, identification);
    write_be16(&mut frame, ip + 6, 0);
    frame[ip + 8] = 64;
    frame[ip + 9] = IP_PROTOCOL_UDP;
    frame[ip + 12..ip + 16].copy_from_slice(&src_ip);
    frame[ip + 16..ip + 20].copy_from_slice(&dst_ip);
    let checksum = internet_checksum(&frame[ip..ip + 20]);
    write_be16(&mut frame, ip + 10, checksum);

    let udp = ip + 20;
    write_be16(&mut frame, udp, src_port);
    write_be16(&mut frame, udp + 2, dst_port);
    write_be16(&mut frame, udp + 4, udp_len as u16);
    write_be16(&mut frame, udp + 6, 0);
    frame[udp + 8..udp + 8 + payload.len()].copy_from_slice(payload);
    let checksum = udp_checksum(src_ip, dst_ip, &frame[udp..udp + udp_len]);
    write_be16(&mut frame, udp + 6, checksum);

    frame
}

fn udp_checksum(src_ip: [u8; 4], dst_ip: [u8; 4], udp_packet: &[u8]) -> u16 {
    let mut sum = 0_u32;
    sum = add_bytes(sum, &src_ip);
    sum = add_bytes(sum, &dst_ip);
    sum += IP_PROTOCOL_UDP as u32;
    sum += udp_packet.len() as u32;
    sum = add_bytes(sum, udp_packet);

    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    let checksum = !(sum as u16);
    if checksum == 0 { 0xFFFF } else { checksum }
}

fn add_bytes(mut sum: u32, bytes: &[u8]) -> u32 {
    let mut i = 0;
    while i + 1 < bytes.len() {
        sum += ((bytes[i] as u32) << 8) | bytes[i + 1] as u32;
        i += 2;
    }

    if i < bytes.len() {
        sum += (bytes[i] as u32) << 8;
    }

    sum
}
