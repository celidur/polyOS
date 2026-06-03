pub const BROADCAST_MAC: [u8; 6] = [0xFF; 6];
pub const ETHERTYPE_ARP: u16 = 0x0806;
pub const ETHERTYPE_IPV4: u16 = 0x0800;
pub const IP_PROTOCOL_ICMP: u8 = 1;
pub const IP_PROTOCOL_UDP: u8 = 17;
pub const UDP_PORT_DHCP_SERVER: u16 = 67;
pub const UDP_PORT_DHCP_CLIENT: u16 = 68;

#[derive(Clone, Copy)]
pub struct Ipv4Packet {
    pub protocol: u8,
    pub ttl: u8,
    pub src: [u8; 4],
    pub dst: [u8; 4],
    pub payload_offset: usize,
    pub payload_len: usize,
}

pub fn ethertype(frame: &[u8]) -> Option<u16> {
    if frame.len() < 14 {
        return None;
    }

    Some(read_be16(frame, 12))
}

pub fn ipv4_packet(frame: &[u8]) -> Option<Ipv4Packet> {
    if ethertype(frame)? != ETHERTYPE_IPV4 || frame.len() < 34 {
        return None;
    }

    let ip_offset = 14;
    if frame[ip_offset] >> 4 != 4 {
        return None;
    }

    let ihl = ((frame[ip_offset] & 0x0F) as usize) * 4;
    if ihl < 20 {
        return None;
    }

    let total_len = read_be16(frame, ip_offset + 2) as usize;
    if total_len < ihl || frame.len() < ip_offset + total_len {
        return None;
    }

    Some(Ipv4Packet {
        protocol: frame[ip_offset + 9],
        ttl: frame[ip_offset + 8],
        src: read_ipv4(frame, ip_offset + 12),
        dst: read_ipv4(frame, ip_offset + 16),
        payload_offset: ip_offset + ihl,
        payload_len: total_len - ihl,
    })
}

pub fn is_dhcp_ports(src: u16, dst: u16) -> bool {
    (src == UDP_PORT_DHCP_SERVER || src == UDP_PORT_DHCP_CLIENT)
        && (dst == UDP_PORT_DHCP_SERVER || dst == UDP_PORT_DHCP_CLIENT)
}

pub fn write_ethernet_header(frame: &mut [u8], dst_mac: [u8; 6], src_mac: [u8; 6], ethertype: u16) {
    frame[0..6].copy_from_slice(&dst_mac);
    frame[6..12].copy_from_slice(&src_mac);
    write_be16(frame, 12, ethertype);
}

pub fn read_ipv4(buf: &[u8], offset: usize) -> [u8; 4] {
    [
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ]
}

pub fn read_be16(buf: &[u8], offset: usize) -> u16 {
    ((buf[offset] as u16) << 8) | buf[offset + 1] as u16
}

pub fn read_be32(buf: &[u8], offset: usize) -> u32 {
    ((buf[offset] as u32) << 24)
        | ((buf[offset + 1] as u32) << 16)
        | ((buf[offset + 2] as u32) << 8)
        | buf[offset + 3] as u32
}

pub fn write_be16(buf: &mut [u8], offset: usize, value: u16) {
    buf[offset] = (value >> 8) as u8;
    buf[offset + 1] = value as u8;
}

pub fn write_be32(buf: &mut [u8], offset: usize, value: u32) {
    buf[offset] = (value >> 24) as u8;
    buf[offset + 1] = (value >> 16) as u8;
    buf[offset + 2] = (value >> 8) as u8;
    buf[offset + 3] = value as u8;
}

pub fn internet_checksum(buf: &[u8]) -> u16 {
    let mut sum = 0_u32;
    let mut i = 0;
    while i + 1 < buf.len() {
        sum += ((buf[i] as u32) << 8) | buf[i + 1] as u32;
        i += 2;
    }

    if i < buf.len() {
        sum += (buf[i] as u32) << 8;
    }

    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !(sum as u16)
}
