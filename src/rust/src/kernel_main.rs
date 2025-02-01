use alloc::{boxed::Box, string::String};

use crate::{
    allocator::{init_heap, serial_print_memory},
    bindings::{
        boot_loadinfo, kernel_init, kernel_init2, process, process_load_switch,
        task_run_first_ever_task,
    },
    devices::pci::pci_read_config,
    entry_point,
    fs::{tmp::TmpFileSystem, FileSystem, ROOT_FS},
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

    // create file

    {
        let mut root_fs = ROOT_FS.lock();

        let tmp_fs = TmpFileSystem::new("/tmp");
        root_fs.mount("/tmp", Box::new(tmp_fs)).unwrap();

        let tmp_fs = TmpFileSystem::new("/");
        root_fs.mount("/", Box::new(tmp_fs)).unwrap();

        serial_println!("root_fs: {:?}", root_fs);

        let res = root_fs.open("/tmp/test.txt", crate::fs::FileMode::Write);
        match res {
            Ok(_) => {
                serial_println!("file exists");
            }
            Err(e) => {
                serial_println!("Error: {:?}", e);
            }
        }

        let mut f = root_fs
            .open("/tmp/test.txt", crate::fs::FileMode::Write)
            .unwrap();
        f.write(b"Hello, World!\n").unwrap();
        f.seek(0, crate::fs::SeekMode::Set).unwrap();
        let mut buf = [0; 14];
        f.read(&mut buf).unwrap();
        assert!(&buf == b"Hello, World!\n");
        root_fs.close(f).unwrap();

        let mut f = root_fs
            .open("/tmp/test.txt", crate::fs::FileMode::Read)
            .unwrap();
        let mut buf = [0; 14];
        f.read(&mut buf).unwrap();

        let s = String::from_utf8(buf.to_vec()).unwrap();
        serial_println!("Read: {:?}", s);
        root_fs.close(f).unwrap();
    }

    let p: *mut *mut process = core::ptr::null_mut();
    let res = unsafe { process_load_switch(c"0:/bin/shell-v2.elf".as_ptr(), p) };
    if res < 0 {
        panic!("Failed to load process");
    }

    unsafe { task_run_first_ever_task() };
}
