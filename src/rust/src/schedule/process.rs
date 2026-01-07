use core::ffi::c_void;

use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use spin::{Mutex, RwLock};

use crate::{
    constant::{
        PROGRAM_VIRTUAL_ADDRESS, USER_PROGRAM_STACK_SIZE, USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END, USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START,
    },
    error::KernelError,
    fs::FileHandle,
    kernel::KERNEL,
    memory::{self, Page, PageDirectory},
    schedule::{loader::elf::{ElfFile, PF_W}},
};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct command_argument {
    pub argument: [::core::ffi::c_char; 512usize],
    pub next: *mut command_argument,
}

use super::task::TaskId;

pub type ProcessId = u32;
pub enum ProcessFileType {
    Elf(ElfFile),
    Binary(Page),
}

#[derive(Debug)]
pub struct ProcessArguments {
    pub args: Vec<String>,
}

pub struct Process {
    pub pid: ProcessId,
    pub fd_table: Vec<Option<Arc<FileHandle>>>,
    pub children: Mutex<Vec<ProcessId>>,
    pub parent: Option<ProcessId>,
    pub entrypoint: u32,
    pub tasks: RwLock<Option<TaskId>>,
    pub filetype: ProcessFileType,
    pub page_directory: PageDirectory,
    pub stack: Page,
    pub start_stack: usize,
    pub heap: Mutex<BTreeMap<u32, Page>>,
}

unsafe impl Send for Process {}
unsafe impl Sync for Process {}

impl Process {
    pub fn new(
        pid: ProcessId,
        parent: Option<ProcessId>,
        filename: &str,
        args: Option<ProcessArguments>,
    ) -> Result<Self, KernelError> {
        let mut process = if let Some(elf) = Self::load_elf(filename) {
            elf
        } else {
            Self::load_binary(filename)?
        };

        process.pid = pid;
        process.parent = parent;

        process.map_memory()?;

        let args = if let Some(args) = args {
            args
        } else {
            ProcessArguments {
                args: vec![filename.to_string()],
            }
        };

        let stack = process.stack.as_mut_slice();
        let mut stack_pointer = stack.len();

        let mut addr = vec![0];
        for arg in args.args.iter().rev() {
            let bytes = arg.as_bytes();
            stack_pointer -= bytes.len() + 1; // +1 for null terminator
            stack[stack_pointer..stack_pointer + bytes.len()]
                .copy_from_slice(bytes);
            stack[stack_pointer + bytes.len()] = 0; // null terminator
            addr.push(USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END + stack_pointer);
        }

        // store argv
        addr.reverse();
        for &arg_addr in addr.iter().rev() {
            stack_pointer -= core::mem::size_of::<*const i8>();
            stack[stack_pointer..stack_pointer + core::mem::size_of::<*const i8>()]
                .copy_from_slice(&arg_addr.to_ne_bytes());
        }

        let argv_ptr = USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END + stack_pointer;
        stack_pointer -= core::mem::size_of::<*const *const i8>();
        stack[stack_pointer..stack_pointer + core::mem::size_of::<*const *const i8>()]
            .copy_from_slice(&argv_ptr.to_ne_bytes());

        // store argc
        stack_pointer -= core::mem::size_of::<i32>();
        stack[stack_pointer..stack_pointer + core::mem::size_of::<i32>()]
            .copy_from_slice(&(args.args.len() as i32).to_ne_bytes());

        process.start_stack = USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END + stack_pointer;

        Ok(process)
    }

    fn load_elf(filename: &str) -> Option<Self> {
        let elf = ElfFile::load(filename).ok()?;
        let entrypoint = elf.header().e_entry;
        let page_directory = PageDirectory::new_4gb(memory::PRESENT)?;
        Some(Self {
            pid: 0,
            fd_table: vec![],
            children: Mutex::new(vec![]),
            parent: None,
            tasks: RwLock::new(None),
            filetype: ProcessFileType::Elf(elf),
            page_directory,
            stack: Page::new(USER_PROGRAM_STACK_SIZE)?,
            start_stack: USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START,
            entrypoint,
            heap: Mutex::new(BTreeMap::new()),
        })
    }

    fn load_binary(filename: &str) -> Result<Self, KernelError> {
        let mut file = KERNEL
            .vfs
            .read()
            .open(filename)
            .map_err(|_| KernelError::Io)?;
        let stat = file.ops.stat().map_err(|_| KernelError::Io)?;

        let mut memory = Page::new(stat.size as usize).ok_or(KernelError::Allocation)?;

        file.ops
            .read(memory.as_mut_slice())
            .map_err(|_| KernelError::Io)?;
        let page_directory =
            PageDirectory::new_4gb(memory::PRESENT).ok_or(KernelError::Allocation)?;
        Ok(Self {
            pid: 0,
            fd_table: vec![],
            children: Mutex::new(vec![]),
            parent: None,
            tasks: RwLock::new(None),
            filetype: ProcessFileType::Binary(memory),
            page_directory,
            stack: Page::new(USER_PROGRAM_STACK_SIZE).ok_or(KernelError::Allocation)?,
            start_stack: USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START,
            entrypoint: PROGRAM_VIRTUAL_ADDRESS as u32,
            heap: Mutex::new(BTreeMap::new()),
        })
    }

    fn map_memory(&mut self) -> Result<(), KernelError> {
        self.page_directory
            .map_page(
                USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END as u32,
                &self.stack,
                memory::PRESENT | memory::WRITABLE | memory::USER_ACCESS,
            )
            .map_err(|_| KernelError::Paging)?;

        match self.filetype {
            ProcessFileType::Elf(ref elf) => {
                for phdr in elf.header().program_headers() {
                    let phdr_phys_adress = elf.phdr_phys_address(phdr);
                    let mut flags = memory::PRESENT | memory::USER_ACCESS;
                    if (phdr.p_flags & PF_W) != 0 {
                        flags |= memory::WRITABLE;
                    }
                    self.page_directory
                        .map_to(
                            PageDirectory::align_address_down(phdr.p_vaddr),
                            PageDirectory::align_address_down(phdr_phys_adress as u32),
                            PageDirectory::align_address(unsafe {
                                phdr_phys_adress.add(phdr.p_memsz as usize)
                            } as u32),
                            flags,
                        )
                        .map_err(|_| KernelError::Paging)?;
                }
            }
            ProcessFileType::Binary(ref memory) => {
                self.page_directory
                    .map_page(
                        PROGRAM_VIRTUAL_ADDRESS as u32,
                        memory,
                        memory::PRESENT | memory::WRITABLE | memory::USER_ACCESS,
                    )
                    .map_err(|_| KernelError::Paging)?;
            }
        }

        Ok(())
    }

    pub fn malloc(&self, size: usize) -> *mut c_void {
        if size == 0 {
            return core::ptr::null_mut();
        }

        let memory = match Page::new(size) {
            Some(page) => page,
            None => return core::ptr::null_mut(),
        };

        let mut heap = self.heap.lock();

        if self
            .page_directory
            .map_page(
                memory.as_ptr() as u32,
                &memory,
                memory::PRESENT | memory::WRITABLE | memory::USER_ACCESS,
            )
            .is_err()
        {
            return core::ptr::null_mut();
        }

        let res = memory.as_ptr() as *mut c_void;

        heap.insert(memory.as_ptr() as u32, memory);

        res
    }

    pub fn free(&self, ptr: *mut c_void) {
        if ptr.is_null() {
            return;
        }

        let mut heap = self.heap.lock();

        let addr = ptr as u32;
        if let Some(page) = heap.remove(&addr) {
            let _ = self.page_directory.map_page(addr, &page, 0);
        }
    }

    pub fn cleanup(&self) {
        for (addr, page) in self.heap.lock().iter() {
            let _ = self.page_directory.map_page(*addr, page, 0);
        }
        self.heap.lock().clear();
    }
}
