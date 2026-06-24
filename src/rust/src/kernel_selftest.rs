use crate::{
    constant::PAGING_PAGE_SIZE,
    kernel::KERNEL,
    memory::{self, Page, PageDirectory},
    schedule::loader::elf::{ElfFile, PF_W},
};

struct Runner {
    passed: u32,
    failed: u32,
}

impl Runner {
    const fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
        }
    }

    fn check(&mut self, name: &str, condition: bool) {
        if condition {
            self.passed += 1;
            serial_println!("[kernel-ok] {}", name);
        } else {
            self.failed += 1;
            serial_println!("[kernel-fail] {}", name);
        }
    }

    fn finish(self) -> u32 {
        serial_println!(
            "kernel selftest: passed={} failed={}",
            self.passed,
            self.failed
        );
        self.failed
    }
}

pub fn run() -> u32 {
    let mut runner = Runner::new();

    test_page(&mut runner);
    test_page_directory_cow(&mut runner);
    test_vfs_devices(&mut runner);
    test_vfs_memfs(&mut runner);
    test_elf_loader(&mut runner);

    runner.finish()
}

fn test_page(runner: &mut Runner) {
    let Some(page) = Page::<u8>::new(64) else {
        runner.check("page allocate", false);
        return;
    };

    runner.check("page allocate", page.len() >= PAGING_PAGE_SIZE);
    runner.check(
        "page zeroed",
        page.as_slice()[0..64].iter().all(|&byte| byte == 0),
    );

    page.as_mut_slice()[7] = 0x5a;
    let Some(copy) = page.copy() else {
        runner.check("page copy", false);
        return;
    };

    runner.check("page copy", copy.as_slice()[7] == 0x5a);
    copy.as_mut_slice()[7] = 0xa5;
    runner.check(
        "page copy independent",
        page.as_slice()[7] == 0x5a && copy.as_slice()[7] == 0xa5,
    );
}

fn test_page_directory_cow(runner: &mut Runner) {
    let Some(directory) = PageDirectory::new_4gb(0) else {
        runner.check("page directory allocate", false);
        return;
    };

    let Some(page) = Page::<u8>::new(16) else {
        runner.check("page directory page allocate", false);
        return;
    };

    let virtual_address = 0x0200_0000;
    let flags = memory::PRESENT | memory::WRITABLE | memory::USER_ACCESS;
    runner.check(
        "page directory map_page",
        directory.map_page(virtual_address, &page, flags).is_ok(),
    );

    let translated = directory.get_physical_address(virtual_address + 3);
    runner.check(
        "page directory translate",
        translated == Ok(page.as_ptr() as u32 + 3),
    );

    let Some(child) = directory.cow_copy() else {
        runner.check("page directory cow_copy", false);
        return;
    };

    runner.check("page directory cow_copy", true);

    let parent_entry = directory.get(virtual_address).unwrap_or(0);
    let child_entry = child.get(virtual_address).unwrap_or(0);
    let cow_flags = memory::PRESENT | memory::USER_ACCESS | memory::COW;

    runner.check(
        "page directory parent cow flags",
        parent_entry & cow_flags == cow_flags && parent_entry & memory::WRITABLE == 0,
    );
    runner.check(
        "page directory child cow flags",
        child_entry & cow_flags == cow_flags && child_entry & memory::WRITABLE == 0,
    );
}

fn test_vfs_devices(runner: &mut Runner) {
    let mut zero = match KERNEL.vfs.read().open("/dev/zero") {
        Ok(file) => file,
        Err(_) => {
            runner.check("vfs open /dev/zero", false);
            return;
        }
    };

    runner.check("vfs open /dev/zero", true);

    let mut buffer = [0xff; 8];
    runner.check(
        "vfs read /dev/zero",
        matches!(zero.ops.read(&mut buffer), Ok(read) if read == buffer.len())
            && buffer.iter().all(|&byte| byte == 0),
    );

    let mut null = match KERNEL.vfs.read().open("/dev/null") {
        Ok(file) => file,
        Err(_) => {
            runner.check("vfs open /dev/null", false);
            return;
        }
    };

    runner.check("vfs open /dev/null", true);

    let data = b"discard";
    runner.check(
        "vfs write /dev/null",
        matches!(null.ops.write(data), Ok(written) if written == data.len()),
    );

    let root_entries = KERNEL.vfs.read().read_dir("/").unwrap_or_default();
    runner.check(
        "vfs root lists /dev mount",
        root_entries.iter().any(|entry| entry == "dev"),
    );
    runner.check(
        "vfs root lists /tmp mount",
        root_entries.iter().any(|entry| entry == "tmp"),
    );
    runner.check(
        "vfs root lists /bin lower fs",
        root_entries.iter().any(|entry| entry == "bin"),
    );
    runner.check(
        "vfs stat /bin lower fs",
        KERNEL
            .vfs
            .read()
            .stat("/bin")
            .is_ok_and(|metadata| metadata.is_dir),
    );
    let overlay_path = "/kernel-selftest-root-overlay";
    let _ = KERNEL.vfs.read().remove(overlay_path);
    runner.check(
        "vfs root overlay mkdir",
        KERNEL.vfs.read().mkdir(overlay_path).is_ok(),
    );
    let root_entries = KERNEL.vfs.read().read_dir("/").unwrap_or_default();
    runner.check(
        "vfs root merges overlay and lower",
        root_entries
            .iter()
            .any(|entry| entry == "kernel-selftest-root-overlay")
            && root_entries.iter().any(|entry| entry == "bin"),
    );
    let _ = KERNEL.vfs.read().remove(overlay_path);
    runner.check(
        "vfs create missing parent",
        matches!(
            KERNEL
                .vfs
                .read()
                .create("/kernel-selftest-missing/file", false),
            Err(crate::fs::FsError::NotFound)
        ),
    );
}

fn test_vfs_memfs(runner: &mut Runner) {
    let path = "/tmp/kernel-selftest.txt";
    let data = b"kernel-vfs";

    let vfs = KERNEL.vfs.read();
    let _ = vfs.remove(path);
    runner.check("vfs memfs create", vfs.create(path, false).is_ok());

    let mut file = match vfs.open(path) {
        Ok(file) => file,
        Err(_) => {
            runner.check("vfs memfs open", false);
            return;
        }
    };

    runner.check("vfs memfs open", true);
    runner.check(
        "vfs memfs write",
        matches!(file.ops.write(data), Ok(written) if written == data.len()),
    );
    runner.check("vfs memfs seek", matches!(file.ops.seek(0), Ok(0)));

    let mut buffer = [0u8; 16];
    runner.check(
        "vfs memfs read",
        matches!(file.ops.read(&mut buffer[..data.len()]), Ok(read) if read == data.len())
            && &buffer[..data.len()] == data,
    );
    runner.check(
        "vfs memfs stat",
        vfs.stat(path)
            .is_ok_and(|metadata| metadata.size >= data.len() as u64),
    );
    runner.check("vfs memfs remove", vfs.remove(path).is_ok());
}

fn test_elf_loader(runner: &mut Runner) {
    let elf = match ElfFile::load("/bin/selftest.elf") {
        Ok(elf) => elf,
        Err(_) => {
            runner.check("elf load /bin/selftest.elf", false);
            return;
        }
    };

    runner.check("elf load /bin/selftest.elf", true);
    runner.check("elf has load segments", !elf.segments().is_empty());

    let Some(writable) = elf
        .segments()
        .iter()
        .find(|segment| segment.flags() & PF_W != 0)
    else {
        runner.check("elf writable segment", false);
        return;
    };

    runner.check("elf writable segment", true);

    let bss_start = writable.page_offset() + writable.file_size();
    let bss_end = (writable.page_offset() + writable.memory_size())
        .min(writable.memory().as_slice().len())
        .min(bss_start + 64);
    let bytes = writable.memory().as_slice();
    runner.check("elf has bss", bss_end > bss_start);
    runner.check(
        "elf bss tail zeroed",
        bss_end > bss_start && bytes[bss_start..bss_end].iter().all(|&byte| byte == 0),
    );
}
