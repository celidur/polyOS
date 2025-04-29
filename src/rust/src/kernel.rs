use lazy_static::lazy_static;

use alloc::{vec, string::ToString, sync::Arc, vec::Vec};
use spin::{Mutex, RwLock};
use uart_16550::SerialPort;

use crate::{
    device::{
        block_dev::{BlockDevice, BlockDeviceError},
        disk::Disk,
    },
    fs::{fat16::Fat16Driver, MemFsDriver, MountOptions, Vfs},
    interrupts, serial_println,
};

#[derive(Debug)]
pub struct Kernel {
    disks: RwLock<Vec<Arc<Mutex<Disk>>>>,
    block_device: RwLock<Vec<Arc<Mutex<dyn BlockDevice>>>>,
    pub vfs: RwLock<Vfs>,
    pub serial_port: Mutex<SerialPort>,
}

lazy_static! {
    pub static ref KERNEL: Arc<Kernel> = Arc::new(Kernel::new());
}

impl Kernel {
    pub fn new() -> Kernel {
        let disk0 = Arc::new(Mutex::new(Disk::new(0x1F0)));

        let disks = RwLock::new(vec![disk0.clone()]);

        let vfs = RwLock::new(Vfs::new());

        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        let serial_port = Mutex::new(serial_port);

        let mut kernel = Kernel {
            disks,
            vfs,
            serial_port,
            block_device: RwLock::new(Vec::new()),
        };

        kernel.register_block_device(disk0);

        kernel
    }

    pub fn init_rootfs(&self) {
        self.vfs
            .write()
            .register_fs_driver("memfs", Arc::new(MemFsDriver));

        self.vfs
            .write()
            .register_fs_driver("fat16", Arc::new(Fat16Driver));

        self.vfs
            .read()
            .mount(
                "/",
                &MountOptions {
                    fs_name: "fat16".to_string(),
                    block_device_id: Some(0),
                },
            )
            .expect("Failed to mount fat16 at /");

        self.vfs
            .read()
            .mount(
                "/tmp",
                &MountOptions {
                    fs_name: "memfs".to_string(),
                    block_device_id: None,
                },
            )
            .expect("Failed to mount memfs at /tmp");
    }

    pub fn register_block_device(&mut self, dev: Arc<Mutex<dyn BlockDevice>>) -> usize {
        self.block_device.write().push(dev);
        self.block_device.read().len() - 1
    }

    pub fn read_sectors(
        &self,
        id: usize,
        lba: u64,
        count: usize,
        buf: &mut [u8],
    ) -> Result<usize, BlockDeviceError> {
        interrupts::without_interrupts(|| {
            self.block_device
                .read()
                .get(id)
                .cloned()
                .ok_or(BlockDeviceError::NotFound)?
                .lock()
                .read_sectors(lba, count, buf)
        })
    }

    pub fn write_sectors(
        &self,
        id: usize,
        lba: u64,
        count: usize,
        buf: &[u8],
    ) -> Result<usize, BlockDeviceError> {
        interrupts::without_interrupts(|| {
            self.block_device
                .read()
                .get(id)
                .cloned()
                .ok_or(BlockDeviceError::NotFound)?
                .lock()
                .write_sectors(lba, count, buf)
        })
    }

    pub fn sync(&self) {
        self.disks.read().iter().for_each(|disk| {
            interrupts::without_interrupts(|| {
                if let Some(mut disk) = disk.try_lock() {
                    let _ = disk.sync();
                } else {
                    serial_println!("sync: disk {:p} already locked, skipping\n", disk);
                }
            })
        });
    }
}
