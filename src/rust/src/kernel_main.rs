use alloc::{string::ToString, sync::Arc};
use spin::Mutex;

use crate::{
    allocator::{init_heap, serial_print_memory},
    bindings::{
        boot_loadinfo, kernel_init, kernel_init2, process, process_load_switch,
        task_run_first_ever_task,
    },
    device::{block_dev::register_block_device, disk::Disk, pci::pci_read_config},
    entry_point,
    fs::{fat16::Fat16Driver, MemFsDriver, MountOptions, VFS},
    serial_println,
};

entry_point!(kernel_main);

fn list_pci_devices() {
    for bus in 0..=255 {
        for device in 0..=31 {
            for function in 0..=7 {
                let vendor_id = unsafe { pci_read_config(bus, device, function, 0x00) } & 0xFFFF;
                let device_id =
                    (unsafe { pci_read_config(bus, device, function, 0x00) } >> 16) & 0xFFFF;
                if vendor_id != 0xFFFF {
                    serial_println!(
                        "PCI Device: Bus {} Device {} Function {} - Vendor: {:#X}, Device: {:#X}",
                        bus,
                        device,
                        function,
                        vendor_id,
                        device_id
                    );
                }
            }
        }
    }
}

fn kernel_main() -> ! {
    unsafe { kernel_init() };

    init_heap();
    unsafe { kernel_init2() };

    unsafe { boot_loadinfo() };

    serial_print_memory();

    list_pci_devices();

    {
        VFS.write()
            .register_fs_driver("memfs", Arc::new(MemFsDriver));
        VFS.write()
            .register_fs_driver("fat16", Arc::new(Fat16Driver));

        let disk = Arc::new(Mutex::new(Disk::new(0x1F0)));

        let dev1_id = register_block_device(disk.clone());

        VFS.read()
            .mount(
                "/",
                &MountOptions {
                    fs_name: "fat16".to_string(),
                    block_device_id: Some(dev1_id),
                },
            )
            .expect("Failed to mount fat16 at /");

        VFS.read()
            .mount(
                "/tmp",
                &MountOptions {
                    fs_name: "memfs".to_string(),
                    block_device_id: None,
                },
            )
            .expect("Failed to mount memfs at /tmp");

        VFS.read()
            .create("/tmp/hello.txt", false)
            .expect("Failed to create file");
        let mut file_handle = VFS
            .read()
            .open("/tmp/hello.txt")
            .expect("Failed to open file");

        let message = b"Hello from the Rust kernel!\n";
        let written = file_handle
            .ops
            .write(message)
            .expect("Failed to write to file");
        assert!(written == message.len());

        file_handle
            .ops
            .seek(0)
            .expect("Failed to seek to start of file");

        let mut buffer = [0u8; 128];
        let read_len = file_handle
            .ops
            .read(&mut buffer)
            .expect("Failed to read from file");
        let read_str = core::str::from_utf8(&buffer[..read_len]).unwrap_or("<invalid utf-8>");

        assert!(read_str == "Hello from the Rust kernel!\n");

        let mut file_handle = VFS.read().open("/hello.txt").expect("Failed to open file");
        let mut buffer = [0u8; 128];
        let read_len = file_handle
            .ops
            .read(&mut buffer)
            .expect("Failed to read from file");

        let read_str = core::str::from_utf8(&buffer[..read_len]).unwrap_or("<invalid utf-8>");
        serial_println!("Read: {}", read_str);

        file_handle
            .ops
            .seek(0)
            .expect("Failed to seek to start of file");

        let written = file_handle
            .ops
            .write(message)
            .expect("Failed to write to file");

        assert!(written == message.len());

        let mut file_handle = VFS.read().open("/hello.txt").expect("Failed to open file");

        file_handle
            .ops
            .seek(0)
            .expect("Failed to seek to start of file");

        let read_len = file_handle
            .ops
            .read(&mut buffer)
            .expect("Failed to read from file");

        let read_str = core::str::from_utf8(&buffer[..read_len]).unwrap_or("<invalid utf-8>");
        serial_println!("Read: {}", read_str);

        let _ = disk.lock().sync();
    }

    let p: *mut *mut process = core::ptr::null_mut();
    let res = unsafe { process_load_switch(c"0:/BIN/SHELL-V2.ELF".as_ptr(), p) };
    if res < 0 {
        panic!("Failed to load process");
    }

    unsafe { task_run_first_ever_task() };
}
