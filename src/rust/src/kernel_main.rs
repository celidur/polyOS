use alloc::{
    format,
    string::{String, ToString},
};

use crate::{
    allocator::{init_heap, serial_print_memory},
    bindings::{
        boot_loadinfo, kernel_init, kernel_init2, process, process_load_switch,
        task_run_first_ever_task,
    },
    device::{bufstream::BufStream, pci::pci_read_config},
    entry_point,
    kernel::KERNEL,
    serial_println,
};

entry_point!(kernel_main);

fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if size < KB {
        format!("{size}B")
    } else if size < MB {
        format!("{}KB", size / KB)
    } else if size < GB {
        format!("{}MB", size / MB)
    } else {
        format!("{}GB", size / GB)
    }
}

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

    KERNEL.init_rootfs();

    serial_print_memory();

    list_pci_devices();

    let storage = BufStream::new(0);
    let fs = fatfs::FileSystem::new(storage, fatfs::FsOptions::new()).unwrap();

    let root_dir = fs.root_dir();
    for r in root_dir.iter() {
        let e = r.unwrap();
        let long = e.long_file_name_as_ucs2_units();
        let name = if let Some(name) = long {
            String::from_utf16_lossy(name)
        } else {
            String::from_utf8_lossy(e.short_file_name_as_bytes()).to_string()
        };
        serial_println!("{:4}  {}", format_file_size(e.len()), name);
    }

    {
        let message = b"Hello from the Rust kernel!\n";

        let mut file_handle = KERNEL
            .vfs
            .read()
            .open("/hello.txt")
            .expect("Failed to open file");
        let mut buffer = [0u8; 128];
        let read_len = file_handle
            .ops
            .read(&mut buffer)
            .expect("Failed to read from file");

        let read_str = core::str::from_utf8(&buffer[..read_len]).unwrap_or("<invalid utf-8>");
        serial_println!("Read: {}", read_str);
        serial_println!("Size: {}", read_len);

        file_handle
            .ops
            .seek(0)
            .expect("Failed to seek to start of file");

        let written = file_handle
            .ops
            .write(message)
            .expect("Failed to write to file");

        assert!(written == message.len());

        let mut file_handle = KERNEL
            .vfs
            .read()
            .open("/hello.txt")
            .expect("Failed to open file");

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
    }

    {
        let mut buffer = [0u8; 128];
        let mut file_handle = KERNEL
            .vfs
            .read()
            .open("/hello.txt")
            .expect("Failed to open file");

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

        let meta = KERNEL
            .vfs
            .read()
            .stat("/hello.txt")
            .expect("Failed to get metadata");

        serial_println!(
            "File: /hello.txt, Size: {}, Mode: {:#o}, UID: {}, GID: {}",
            format_file_size(meta.size),
            meta.mode,
            meta.uid,
            meta.gid
        );

        let dir = KERNEL
            .vfs
            .read()
            .read_dir("/")
            .expect("Failed to get metadata");

        serial_println!("Directory: /");
        for entry in dir {
            serial_println!("  {}", entry);
        }
    }

    let p: *mut *mut process = core::ptr::null_mut();
    let res = unsafe { process_load_switch(c"0:/bin/shell-v2.elf".as_ptr(), p) };
    if res < 0 {
        panic!("Failed to load process");
    }

    unsafe { task_run_first_ever_task() };
}
