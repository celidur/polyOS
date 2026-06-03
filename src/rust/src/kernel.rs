use core::arch::asm;

use lazy_static::lazy_static;

use alloc::{string::ToString, sync::Arc};
use spin::RwLock;

use crate::{
    device::{
        driver::{probe_stage, DeviceProbeStage},
        disk::DISK_DRIVER,
    },
    fs::{DevFsDriver, FatDriver, MemFsDriver, MountOptions, Vfs},
    interrupts,
    memory::{self, PageDirectory},
    schedule::{process_manager::ProcessManager, task_manager::TaskManager},
};

pub struct Kernel {
    process_manager: RwLock<ProcessManager>,
    task_manager: RwLock<TaskManager>,
    pub vfs: RwLock<Vfs>,
    kernel_page_directory: PageDirectory,
}

lazy_static! {
    pub static ref KERNEL: Arc<Kernel> = Arc::new(Kernel::new());
}

impl Kernel {
    pub fn new() -> Self {
        let vfs = RwLock::new(Vfs::new());

        let kernel_page_directory = match PageDirectory::new_4gb(
            memory::WRITABLE | memory::PRESENT | memory::USER_ACCESS,
        ) {
            Some(pd) => pd,
            None => panic!("Failed to create kernel page directory"),
        };

        let kernel = Kernel {
            vfs,
            process_manager: RwLock::new(ProcessManager::new()),
            task_manager: RwLock::new(TaskManager::new()),
            kernel_page_directory,
        };

        kernel.probe_devices(DeviceProbeStage::Early);
        kernel.probe_devices(DeviceProbeStage::Normal);

        kernel.init_rootfs();

        kernel
    }

    fn init_rootfs(&self) {
        self.vfs
            .write()
            .register_fs_driver("memfs", Arc::new(MemFsDriver));

        self.vfs
            .write()
            .register_fs_driver("devfs", Arc::new(DevFsDriver));

        self.vfs
            .write()
            .register_fs_driver("fat", Arc::new(FatDriver));

        self.vfs
            .read()
            .mount(
                "/",
                &MountOptions {
                    fs_name: "fat".to_string(),
                    block_device_id: Some(
                        DISK_DRIVER
                            .block_device_id()
                            .expect("disk block device not probed"),
                    ),
                },
            )
            .expect("Failed to mount fat at /");

        self.vfs
            .read()
            .mount(
                "/dev",
                &MountOptions {
                    fs_name: "devfs".to_string(),
                    block_device_id: None,
                },
            )
            .expect("Failed to mount devfs at /dev");

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

    fn probe_devices(&self, stage: DeviceProbeStage) {
        probe_stage(stage);
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

    pub fn kernel_page(&self) {
        interrupts::without_interrupts(|| {
            self.kernel_registers();
            self.kernel_page_directory.switch();
        });
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
