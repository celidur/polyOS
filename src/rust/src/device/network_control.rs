//! Device control endpoint. Packet transport remains in net and the NIC driver.
use super::control::{POLYOS_IOCTL_NET_DHCP_DISCOVER, POLYOS_IOCTL_NET_INFO};
use crate::{
    fs::{FileHandle, FileMetadata, FileOps, FsError},
    memory::PageDirectory,
    memory::user::copy_to_user,
    net,
};
use alloc::boxed::Box;
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

struct NetworkControl;
impl FileOps for NetworkControl {
    fn read(&mut self, _: &mut [u8]) -> Result<usize, FsError> {
        Err(FsError::Unsupported)
    }
    fn write(&mut self, _: &[u8]) -> Result<usize, FsError> {
        Err(FsError::Unsupported)
    }
    fn seek(&mut self, _: usize) -> Result<usize, FsError> {
        Err(FsError::Unsupported)
    }
    fn stat(&self) -> Result<FileMetadata, FsError> {
        Ok(FileMetadata {
            mode: 0o666,
            ..FileMetadata::default()
        })
    }
    fn ioctl(&mut self, request: u32, arg: u32, directory: &PageDirectory) -> Result<u32, FsError> {
        match request {
            POLYOS_IOCTL_NET_INFO => {
                if arg == 0 {
                    return Err(FsError::BadAddress);
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

                copy_to_user(
                    directory,
                    arg,
                    &stat as *const NetworkStat as *const u8,
                    core::mem::size_of::<NetworkStat>() as u32,
                )
                .map_err(|_| FsError::BadAddress)?;
                Ok(0)
            }
            POLYOS_IOCTL_NET_DHCP_DISCOVER => net::send_dhcp_discover()
                .map(|_| 0)
                .map_err(|_| FsError::IoError),
            _ => Err(FsError::Unsupported),
        }
    }
}
fn open() -> FileHandle {
    FileHandle::new(Box::new(NetworkControl))
}
crate::register_device_node!(NETWORK_CONTROL_NODE, ["net0"], open);
