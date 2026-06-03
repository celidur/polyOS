use alloc::vec::Vec;

use super::packet::{
    BROADCAST_MAC, ETHERTYPE_ARP, ETHERTYPE_IPV4, ethertype, read_be16, read_ipv4, write_be16,
    write_ethernet_header,
};

#[derive(Clone, Copy)]
pub struct ArpPacket {
    pub operation: u16,
    pub sender_mac: [u8; 6],
    pub sender_ip: [u8; 4],
    pub target_ip: [u8; 4],
}

pub fn parse(frame: &[u8]) -> Option<ArpPacket> {
    if ethertype(frame)? != ETHERTYPE_ARP || frame.len() < 42 {
        return None;
    }

    let arp = 14;
    if read_be16(frame, arp) != 1
        || read_be16(frame, arp + 2) != ETHERTYPE_IPV4
        || frame[arp + 4] != 6
        || frame[arp + 5] != 4
    {
        return None;
    }

    Some(ArpPacket {
        operation: read_be16(frame, arp + 6),
        sender_mac: [
            frame[arp + 8],
            frame[arp + 9],
            frame[arp + 10],
            frame[arp + 11],
            frame[arp + 12],
            frame[arp + 13],
        ],
        sender_ip: read_ipv4(frame, arp + 14),
        target_ip: read_ipv4(frame, arp + 24),
    })
}

pub fn build_request(local_mac: [u8; 6], local_ip: [u8; 4], target_ip: [u8; 4]) -> Vec<u8> {
    let mut frame = vec![0; 42];
    write_ethernet_header(&mut frame, BROADCAST_MAC, local_mac, ETHERTYPE_ARP);

    let arp = 14;
    write_be16(&mut frame, arp, 1);
    write_be16(&mut frame, arp + 2, ETHERTYPE_IPV4);
    frame[arp + 4] = 6;
    frame[arp + 5] = 4;
    write_be16(&mut frame, arp + 6, 1);
    frame[arp + 8..arp + 14].copy_from_slice(&local_mac);
    frame[arp + 14..arp + 18].copy_from_slice(&local_ip);
    frame[arp + 18..arp + 24].fill(0);
    frame[arp + 24..arp + 28].copy_from_slice(&target_ip);

    frame
}
