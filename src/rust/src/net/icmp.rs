use alloc::vec::Vec;

use super::packet::{
    ETHERTYPE_IPV4, IP_PROTOCOL_ICMP, internet_checksum, ipv4_packet, read_be16, write_be16,
    write_ethernet_header,
};

const ICMP_ECHO_REPLY: u8 = 0;
const ICMP_ECHO_REQUEST: u8 = 8;
pub const ECHO_PAYLOAD_BYTES: usize = 56;
pub const ECHO_PACKET_BYTES: usize = 8 + ECHO_PAYLOAD_BYTES;
const ICMP_PAYLOAD: [u8; ECHO_PAYLOAD_BYTES] = [0x50; ECHO_PAYLOAD_BYTES];

#[derive(Clone, Copy)]
pub struct EchoReply {
    pub src_ip: [u8; 4],
    pub ttl: u8,
    pub bytes: u16,
    pub id: u16,
    pub sequence: u16,
}

pub fn build_echo_request(
    src_mac: [u8; 6],
    dst_mac: [u8; 6],
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    id: u16,
    sequence: u16,
) -> Vec<u8> {
    let icmp_len = 8 + ICMP_PAYLOAD.len();
    let ip_len = 20 + icmp_len;
    let mut frame = vec![0; 14 + ip_len];

    write_ethernet_header(&mut frame, dst_mac, src_mac, ETHERTYPE_IPV4);

    let ip = 14;
    frame[ip] = 0x45;
    write_be16(&mut frame, ip + 2, ip_len as u16);
    write_be16(&mut frame, ip + 4, sequence);
    write_be16(&mut frame, ip + 6, 0);
    frame[ip + 8] = 64;
    frame[ip + 9] = IP_PROTOCOL_ICMP;
    frame[ip + 12..ip + 16].copy_from_slice(&src_ip);
    frame[ip + 16..ip + 20].copy_from_slice(&dst_ip);
    let ip_checksum = internet_checksum(&frame[ip..ip + 20]);
    write_be16(&mut frame, ip + 10, ip_checksum);

    let icmp = ip + 20;
    frame[icmp] = ICMP_ECHO_REQUEST;
    frame[icmp + 1] = 0;
    write_be16(&mut frame, icmp + 4, id);
    write_be16(&mut frame, icmp + 6, sequence);
    frame[icmp + 8..icmp + 8 + ICMP_PAYLOAD.len()].copy_from_slice(&ICMP_PAYLOAD);
    let icmp_checksum = internet_checksum(&frame[icmp..icmp + icmp_len]);
    write_be16(&mut frame, icmp + 2, icmp_checksum);

    frame
}

pub fn parse_echo_reply(frame: &[u8]) -> Option<EchoReply> {
    let ipv4 = ipv4_packet(frame)?;
    if ipv4.protocol != IP_PROTOCOL_ICMP || ipv4.payload_len < 8 {
        return None;
    }

    let icmp = ipv4.payload_offset;
    if frame[icmp] != ICMP_ECHO_REPLY || frame[icmp + 1] != 0 {
        return None;
    }

    Some(EchoReply {
        src_ip: ipv4.src,
        ttl: ipv4.ttl,
        bytes: ipv4.payload_len as u16,
        id: read_be16(frame, icmp + 4),
        sequence: read_be16(frame, icmp + 6),
    })
}
