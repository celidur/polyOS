use core::{
    alloc::{Allocator, Layout},
    ffi::c_void,
    ptr::{NonNull, null_mut},
};

use alloc::{
    alloc::Global,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use spin::{Mutex, RwLock};

use crate::{
    bindings::{
        self, PAGING_ACCESS_FROM_ALL, PAGING_IS_PRESENT, PAGING_IS_WRITABLE, PAGING_PAGE_SIZE,
        PROGRAM_VIRTUAL_ADDRESS, USER_PROGRAM_STACK_SIZE, USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END,
        page_t, paging_align_address, paging_align_to_lower_page, paging_free_4gb, paging_map_to,
        paging_new_4gb,
    },
    error::KernelError,
    fs::FileHandle,
    kernel::KERNEL,
    loader::elf::{ElfFile, PF_W},
    memory::{AllocationHeader, PAGE_SIZE},
};

use super::task::TaskId;

pub type ProcessId = u32;
pub enum ProcessFileType {
    Elf(ElfFile),
    Binary(Vec<u8>),
}

pub struct ProcessArguments {
    pub args: Vec<String>,
}

pub struct Process {
    pub pid: ProcessId,
    pub fd_table: Vec<Option<Arc<FileHandle>>>,
    pub children: Mutex<Vec<ProcessId>>,
    pub parent: Option<ProcessId>,
    // pub memory_map:     MemoryMap,
    pub entrypoint: u32,
    pub tasks: RwLock<Option<TaskId>>,
    pub filetype: ProcessFileType,
    pub args: bindings::process_argument,
    pub page_directory: bindings::page_t,
    pub stack: Vec<u8>,
    pub heap: Mutex<Vec<AllocationHeader>>,
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

        process.allocate_stack()?;
        process.map_memory()?;

        let args = if let Some(args) = args {
            args
        } else {
            ProcessArguments {
                args: vec![filename.to_string()],
            }
        };

        let mut process_args = bindings::process_argument {
            argv: null_mut(),
            argc: 0,
        };

        process_args.argc = args.args.len() as i32;
        process_args.argv = process
            .malloc((args.args.len() + 1) * core::mem::size_of::<*const i8>())
            as *mut *mut i8;
        let mut args_ptr = process_args.argv;
        for arg in args.args.iter() {
            let arg_ptr = process.malloc(arg.len() + 1) as *mut i8;
            unsafe {
                core::ptr::copy_nonoverlapping(arg.as_ptr() as *const i8, arg_ptr, arg.len());
                // write null terminator at the end the size is len + 1
                let last = arg_ptr.add(arg.len());
                *last = 0;
                *args_ptr = arg_ptr;
                args_ptr = args_ptr.add(1);
            }
        }
        unsafe {
            *args_ptr = null_mut();
        }

        process.args = process_args;

        Ok(process)
    }

    fn load_elf(filename: &str) -> Option<Self> {
        let elf = ElfFile::load(filename).ok()?;
        let entrypoint = elf.header().e_entry;
        let page_directory = unsafe { paging_new_4gb(PAGING_IS_PRESENT as u8) } as bindings::page_t;
        Some(Self {
            pid: 0,
            fd_table: vec![],
            children: Mutex::new(vec![]),
            parent: None,
            tasks: RwLock::new(None),
            filetype: ProcessFileType::Elf(elf),
            args: bindings::process_argument {
                argv: null_mut(),
                argc: 0,
            },
            page_directory,
            stack: vec![],
            entrypoint,
            heap: Mutex::new(vec![]),
        })
    }

    fn allocate_stack(&mut self) -> Result<(), KernelError> {
        let layout =
            Layout::from_size_align(USER_PROGRAM_STACK_SIZE as usize, PAGING_PAGE_SIZE as usize)
                .map_err(|_| KernelError::Allocation)?;
        let ptr = Global
            .allocate_zeroed(layout)
            .map_err(|_| KernelError::Allocation)?;
        self.stack = unsafe {
            Vec::from_raw_parts(
                ptr.as_ptr() as *mut u8,
                USER_PROGRAM_STACK_SIZE as usize,
                USER_PROGRAM_STACK_SIZE as usize,
            )
        };
        Ok(())
    }

    fn load_binary(filename: &str) -> Result<Self, KernelError> {
        let mut file = KERNEL
            .vfs
            .read()
            .open(filename)
            .map_err(|_| KernelError::Io)?;
        let stat = file.ops.stat().map_err(|_| KernelError::Io)?;

        let layout = Layout::from_size_align(stat.size as usize, PAGING_PAGE_SIZE as usize)
            .map_err(|_| KernelError::Allocation)?;
        let ptr = Global
            .allocate_zeroed(layout)
            .map_err(|_| KernelError::Allocation)?;

        let mut memory = unsafe {
            Vec::from_raw_parts(
                ptr.as_ptr() as *mut u8,
                stat.size as usize,
                stat.size as usize,
            )
        };
        file.ops.read(&mut memory).map_err(|_| KernelError::Io)?;
        let page_directory = unsafe { paging_new_4gb(PAGING_IS_PRESENT as u8) } as bindings::page_t;
        Ok(Self {
            pid: 0,
            fd_table: vec![],
            children: Mutex::new(vec![]),
            parent: None,
            tasks: RwLock::new(None),
            filetype: ProcessFileType::Binary(memory),
            args: bindings::process_argument {
                argv: null_mut(),
                argc: 0,
            },
            page_directory,
            stack: vec![],
            entrypoint: PROGRAM_VIRTUAL_ADDRESS,
            heap: Mutex::new(vec![]),
        })
    }

    fn map_memory(&mut self) -> Result<(), KernelError> {
        let res = unsafe {
            paging_map_to(
                self.page_directory as *mut page_t,
                USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END as *mut c_void,
                self.stack.as_ptr() as *mut c_void,
                paging_align_address(
                    self.stack
                        .as_ptr()
                        .add(USER_PROGRAM_STACK_SIZE as usize + 1)
                        as *mut c_void,
                ),
                (PAGING_IS_PRESENT | PAGING_IS_WRITABLE | PAGING_ACCESS_FROM_ALL) as u8,
            )
        };
        if res < 0 {
            return Err(KernelError::Paging);
        }

        match self.filetype {
            ProcessFileType::Elf(ref elf) => {
                for phdr in elf.header().program_headers() {
                    let phdr_phys_adress = elf.phdr_phys_address(phdr);
                    let mut flags = PAGING_IS_PRESENT | PAGING_ACCESS_FROM_ALL;
                    if (phdr.p_flags & PF_W) != 0 {
                        flags |= PAGING_IS_WRITABLE;
                    }
                    let res = unsafe {
                        paging_map_to(
                            self.page_directory as *mut page_t,
                            paging_align_to_lower_page(phdr.p_vaddr as *mut c_void),
                            paging_align_to_lower_page(phdr_phys_adress as *mut c_void),
                            paging_align_address(
                                phdr_phys_adress.add(phdr.p_memsz as usize) as *mut c_void
                            ),
                            flags as u8,
                        )
                    };
                    if res < 0 {
                        return Err(KernelError::Paging);
                    }
                }
            }
            ProcessFileType::Binary(ref memory) => {
                let res = unsafe {
                    paging_map_to(
                        self.page_directory as *mut page_t,
                        PROGRAM_VIRTUAL_ADDRESS as *mut c_void,
                        memory.as_ptr() as *mut c_void,
                        paging_align_address(memory.as_ptr().add(memory.len() + 1) as *mut c_void),
                        (PAGING_IS_PRESENT | PAGING_IS_WRITABLE | PAGING_ACCESS_FROM_ALL) as u8,
                    )
                };
                if res < 0 {
                    return Err(KernelError::Paging);
                }
            }
        }
        Ok(())
    }

    pub fn malloc(&self, size: usize) -> *mut c_void {
        if size == 0 {
            return core::ptr::null_mut();
        }

        let size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let layout = match Layout::from_size_align(size, PAGE_SIZE) {
            Ok(layout) => layout,
            Err(_) => return core::ptr::null_mut(),
        };

        let ptr = Global.allocate_zeroed(layout);
        match ptr {
            Ok(ptr) => {
                let raw_ptr = ptr.as_ptr() as *mut c_void;

                let a = AllocationHeader::new(raw_ptr as u32, size, PAGE_SIZE);

                let mut heap = self.heap.lock();
                heap.push(a);

                let res = unsafe {
                    paging_map_to(
                        self.page_directory as *mut u32,
                        raw_ptr,
                        raw_ptr,
                        paging_align_address(raw_ptr.add(size)),
                        (PAGING_IS_PRESENT | PAGING_IS_WRITABLE | PAGING_ACCESS_FROM_ALL) as u8,
                    )
                };
                if res < 0 {
                    return core::ptr::null_mut();
                }

                raw_ptr
            }
            Err(_) => core::ptr::null_mut(),
        }
    }

    pub fn free(&self, ptr: *mut c_void) {
        if ptr.is_null() {
            return;
        }

        let mut heap = self.heap.lock();
        let mut index = None;
        for (i, header) in heap.iter().enumerate() {
            if header.ptr == ptr as u32 {
                index = Some(i);
                break;
            }
        }

        if let Some(i) = index {
            let header = heap.remove(i);
            if let Ok(layout) = Layout::from_size_align(header.size, header.alignment) {
                let raw_ptr = header.ptr as *mut u8;
                unsafe {
                    Global.deallocate(NonNull::new_unchecked(raw_ptr), layout);
                }

                unsafe {
                    paging_map_to(
                        self.page_directory as *mut u32,
                        raw_ptr as *mut c_void,
                        raw_ptr as *mut c_void,
                        paging_align_address(raw_ptr.add(header.size) as *mut c_void),
                        0,
                    )
                };
            }
        }
    }

    pub fn cleanup(&self) {
        for header in self.heap.lock().iter() {
            let layout = Layout::from_size_align(header.size, header.alignment).unwrap();
            let raw_ptr = header.ptr as *mut u8;
            unsafe {
                Global.deallocate(NonNull::new_unchecked(raw_ptr), layout);
            }
        }
        self.heap.lock().clear();

        unsafe { paging_free_4gb(self.page_directory as *mut u32) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_terminate() {}
