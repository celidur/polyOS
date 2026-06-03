use alloc::vec::Vec;

use super::{
    packet::{read_be16, read_ipv4, write_be16},
    udp,
};

pub const DNS_PORT: u16 = 53;

const TYPE_A: u16 = 1;
const CLASS_IN: u16 = 1;
const FLAG_QUERY_RECURSION_DESIRED: u16 = 0x0100;
const FLAG_RESPONSE: u16 = 0x8000;
const MAX_NAME_LEN: usize = 253;

#[derive(Clone, Copy)]
pub struct DnsResponse {
    pub id: u16,
    pub src_ip: [u8; 4],
    pub dst_port: u16,
    pub answer: Option<[u8; 4]>,
    pub rcode: u8,
}

pub fn build_query(name: &str, id: u16) -> Option<Vec<u8>> {
    let name = name.trim_end_matches('.');
    if name.is_empty() || name.len() > MAX_NAME_LEN {
        return None;
    }

    let mut packet = vec![0; 12];
    write_be16(&mut packet, 0, id);
    write_be16(&mut packet, 2, FLAG_QUERY_RECURSION_DESIRED);
    write_be16(&mut packet, 4, 1);

    for label in name.split('.') {
        if label.is_empty() || label.len() > 63 || !is_valid_label(label) {
            return None;
        }

        packet.push(label.len() as u8);
        packet.extend_from_slice(label.as_bytes());
    }

    packet.push(0);
    packet.extend_from_slice(&[0, TYPE_A as u8, 0, CLASS_IN as u8]);

    Some(packet)
}

pub fn parse_response(frame: &[u8]) -> Option<DnsResponse> {
    let udp = udp::parse(frame)?;
    if udp.src_port != DNS_PORT {
        return None;
    }

    let payload = &frame[udp.payload_offset..udp.payload_offset + udp.payload_len];
    if payload.len() < 12 {
        return None;
    }

    let flags = read_be16(payload, 2);
    if flags & FLAG_RESPONSE == 0 {
        return None;
    }

    let question_count = read_be16(payload, 4) as usize;
    let answer_count = read_be16(payload, 6) as usize;
    let mut offset = 12;

    for _ in 0..question_count {
        offset = skip_name(payload, offset)?;
        if offset + 4 > payload.len() {
            return None;
        }
        offset += 4;
    }

    let mut answer = None;
    for _ in 0..answer_count {
        offset = skip_name(payload, offset)?;
        if offset + 10 > payload.len() {
            return None;
        }

        let record_type = read_be16(payload, offset);
        let record_class = read_be16(payload, offset + 2);
        let record_len = read_be16(payload, offset + 8) as usize;
        offset += 10;

        if offset + record_len > payload.len() {
            return None;
        }

        if record_type == TYPE_A && record_class == CLASS_IN && record_len == 4 && answer.is_none()
        {
            answer = Some(read_ipv4(payload, offset));
        }

        offset += record_len;
    }

    Some(DnsResponse {
        id: read_be16(payload, 0),
        src_ip: udp.src_ip,
        dst_port: udp.dst_port,
        answer,
        rcode: (flags & 0x000F) as u8,
    })
}

fn skip_name(packet: &[u8], mut offset: usize) -> Option<usize> {
    loop {
        if offset >= packet.len() {
            return None;
        }

        let len = packet[offset];
        if len & 0xC0 == 0xC0 {
            if offset + 1 >= packet.len() {
                return None;
            }
            return Some(offset + 2);
        }

        if len & 0xC0 != 0 {
            return None;
        }

        if len == 0 {
            return Some(offset + 1);
        }

        offset += 1 + len as usize;
        if offset > packet.len() {
            return None;
        }
    }
}

fn is_valid_label(label: &str) -> bool {
    label.bytes().all(|byte| {
        byte == b'-'
            || byte == b'_'
            || (byte >= b'0' && byte <= b'9')
            || (byte >= b'A' && byte <= b'Z')
            || (byte >= b'a' && byte <= b'z')
    })
}
