use core::arch::asm;

use lazy_static::lazy_static;

use alloc::{collections::VecDeque, string::ToString, sync::Arc, vec::Vec};
use spin::{Mutex, RwLock};
use uart_16550::SerialPort;

use crate::{
    device::{
        block_dev::{BlockDevice, BlockDeviceError},
        disk::Disk,
        screen::{GraphicVga, ScreenMode, TextMode, TextVga, Vga},
    },
    fs::{MemFsDriver, MountOptions, Vfs, fat::FatDriver},
    interrupts,
    memory::{self, PageDirectory},
    schedule::{process_manager::ProcessManager, task_manager::TaskManager},
};

pub struct Kernel<'a> {
    disks: RwLock<Vec<Arc<Mutex<Disk>>>>,
    block_device: RwLock<Vec<Arc<Mutex<dyn BlockDevice>>>>,
    serial_port: Mutex<SerialPort>,
    vga: RwLock<Vga<'a>>,
    process_manager: RwLock<ProcessManager>,
    task_manager: RwLock<TaskManager>,
    keyboard: RwLock<VecDeque<u8>>,
    pub vfs: RwLock<Vfs>,
    kernel_page_directory: RwLock<Option<PageDirectory>>,
}

lazy_static! {
    pub static ref KERNEL: Arc<Kernel<'static>> = Arc::new(Kernel::new());
}

impl Kernel<'_> {
    pub fn new() -> Self {
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
            vga: RwLock::new(Vga::new(ScreenMode::Text(TextMode::Text90x60))),
            process_manager: RwLock::new(ProcessManager::new()),
            keyboard: RwLock::new(VecDeque::new()),
            task_manager: RwLock::new(TaskManager::new()),
            kernel_page_directory: RwLock::new(None),
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
            .register_fs_driver("fat", Arc::new(FatDriver));

        self.vfs
            .read()
            .mount(
                "/",
                &MountOptions {
                    fs_name: "fat".to_string(),
                    block_device_id: Some(0),
                },
            )
            .expect("Failed to mount fat at /");

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

    pub fn init_page(&self) {
        let kernel_page_directory = match PageDirectory::new_4gb(
            memory::WRITABLE | memory::PRESENT | memory::USER_ACCESS,
        ) {
            Some(pd) => pd,
            None => panic!("Failed to create kernel page directory"),
        };
        *self.kernel_page_directory.write() = Some(kernel_page_directory);
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
                let _ = disk.lock().sync();
            })
        });
    }

    pub fn serial(&self, args: ::core::fmt::Arguments) {
        use crate::interrupts;
        use core::fmt::Write;

        interrupts::without_interrupts(|| {
            self.serial_port
                .lock()
                .write_fmt(args)
                .expect("Printing to serial failed")
        });
    }

    pub fn set_mode(&self, mode: ScreenMode) {
        interrupts::without_interrupts(|| {
            let mut vga = self.vga.write();
            vga.set_mode(mode);
        });
    }

    pub fn with_text<F, R>(&self, f: F) -> R
    where
        F: FnOnce(Option<&mut TextVga<'_>>) -> R,
    {
        interrupts::without_interrupts(|| {
            let mut vga = self.vga.write();
            let text = vga.get_text_vga();
            f(text)
        })
    }

    pub fn with_graphic<F, R>(&self, f: F) -> R
    where
        F: FnOnce(Option<&mut GraphicVga<'_>>) -> R,
    {
        interrupts::without_interrupts(|| {
            let mut vga = self.vga.write();
            let graphic = vga.get_graphic_vga();
            f(graphic)
        })
    }

    pub fn with_process_manager<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut ProcessManager) -> R,
    {
        interrupts::without_interrupts(|| {
            let process_manager = &mut *self.process_manager.write();
            f(process_manager)
        })
    }

    pub fn with_task_manager<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut TaskManager) -> R,
    {
        interrupts::without_interrupts(|| {
            let task_manager = &mut *self.task_manager.write();
            f(task_manager)
        })
    }

    pub fn keyboard_push(&self, c: u8) {
        interrupts::without_interrupts(|| {
            let mut keyboard = self.keyboard.write();
            keyboard.push_back(c);
        });
    }

    pub fn keyboard_pop(&self) -> Option<u8> {
        interrupts::without_interrupts(|| {
            let mut keyboard = self.keyboard.write();
            keyboard.pop_front()
        })
    }

    pub fn kernel_page(&self) {
        self.kernel_registers();
        match &*self.kernel_page_directory.read() {
            Some(pd) => pd.switch(),
            None => panic!("Kernel page directory not initialized"),
        }
    }

    fn kernel_registers(&self) {
        unsafe {
            asm!(
                "
                mov ax, 0x10
                mov ds, ax
                mov es, ax
                mov fs, ax
                mov gs, ax
                ",
                options(nomem, nostack, preserves_flags)
            );
        }
    }
}
