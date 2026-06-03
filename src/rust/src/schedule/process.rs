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
        PAGING_PAGE_SIZE, PROGRAM_VIRTUAL_ADDRESS, USER_PROGRAM_STACK_SIZE,
        USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END, USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START,
    },
    error::KernelError,
    fs::{FileHandle, FileMetadata, FsError, Pipe, PipeEnd},
    kernel::KERNEL,
    memory::{self, Page, PageDirectory},
    schedule::loader::elf::{ElfFile, PF_W},
};

use super::task::TaskId;

pub type ProcessId = u32;
const FIRST_PROCESS_FD: usize = 3;
const MAX_PROCESS_FD: usize = 128;

#[derive(Clone)]
pub enum ProcessDescriptor {
    File(Arc<Mutex<FileHandle>>),
    Socket(Arc<Mutex<SocketHandle>>),
    Pipe {
        pipe: Arc<Mutex<Pipe>>,
        end: PipeEnd,
    },
}

pub struct SocketHandle {
    id: u32,
    refs: usize,
}

impl SocketHandle {
    pub fn new(id: u32) -> Self {
        Self { id, refs: 1 }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    fn duplicate(&mut self) {
        self.refs = self.refs.saturating_add(1);
    }

    fn close(&mut self) -> Option<u32> {
        self.refs = self.refs.saturating_sub(1);
        (self.refs == 0).then_some(self.id)
    }
}

impl ProcessDescriptor {
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, FsError> {
        match self {
            Self::File(file) => file.lock().ops.read(buf),
            Self::Pipe { pipe, end } => pipe.lock().read(*end, buf).map_err(|_| FsError::IoError),
            Self::Socket(_) => Err(FsError::Unsupported),
        }
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, FsError> {
        match self {
            Self::File(file) => file.lock().ops.write(buf),
            Self::Pipe { pipe, end } => pipe.lock().write(*end, buf).map_err(|_| FsError::IoError),
            Self::Socket(_) => Err(FsError::Unsupported),
        }
    }

    pub fn seek(&self, pos: usize) -> Result<usize, FsError> {
        match self {
            Self::File(file) => file.lock().ops.seek(pos),
            _ => Err(FsError::Unsupported),
        }
    }

    pub fn ioctl(&self, request: u32, arg: u32, directory: &PageDirectory) -> Result<u32, FsError> {
        match self {
            Self::File(file) => file.lock().ops.ioctl(request, arg, directory),
            _ => Err(FsError::Unsupported),
        }
    }

    pub fn stat(&self) -> Result<FileMetadata, FsError> {
        match self {
            Self::File(file) => file.lock().ops.stat(),
            _ => Err(FsError::Unsupported),
        }
    }

    pub fn duplicate_for_fd_table(&self) -> Self {
        match self {
            Self::File(file) => Self::File(file.clone()),
            Self::Socket(socket) => {
                socket.lock().duplicate();
                Self::Socket(socket.clone())
            }
            Self::Pipe { pipe, end } => {
                pipe.lock().clone_end(*end);
                Self::Pipe {
                    pipe: pipe.clone(),
                    end: *end,
                }
            }
        }
    }

    pub fn close(self) {
        match self {
            Self::File(_) => {}
            Self::Socket(socket) => {
                if let Some(socket_id) = socket.lock().close() {
                    let _ = crate::net::socket_close(socket_id);
                }
            }
            Self::Pipe { pipe, end } => pipe.lock().close_end(end),
        }
    }
}

#[derive(Clone)]
pub enum ProcessFileType {
    Elf(ElfFile),
    Binary(Page<u8>),
}

#[derive(Debug)]
pub struct ProcessArguments {
    pub args: Vec<String>,
}

pub struct Process {
    pub pid: ProcessId,
    pub fd_table: Mutex<Vec<Option<ProcessDescriptor>>>,
    pub children: Mutex<Vec<ProcessId>>,
    pub parent: Option<ProcessId>,
    pub entrypoint: u32,
    pub tasks: RwLock<Option<TaskId>>,
    pub filetype: ProcessFileType,
    pub page_directory: PageDirectory,
    pub stack: Page<u8>,
    pub start_stack: usize,
    pub heap: Mutex<BTreeMap<u32, Page<u8>>>,
    cow_pages: Mutex<BTreeMap<u32, Page<u8>>>,
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

        serial_println!("Process {} args: {:?}", pid, args.args);

        let stack = process.stack.as_mut_slice();
        let mut stack_pointer = stack.len();

        let mut argv = Vec::with_capacity(args.args.len() + 1);
        for arg in args.args.iter().rev() {
            let bytes = arg.as_bytes();
            if stack_pointer < bytes.len() + 1 {
                return Err(KernelError::Allocation);
            }
            stack_pointer -= bytes.len() + 1; // +1 for null terminator
            stack[stack_pointer..stack_pointer + bytes.len()].copy_from_slice(bytes);
            stack[stack_pointer + bytes.len()] = 0; // null terminator
            argv.push(USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END + stack_pointer);
        }

        argv.reverse();
        argv.push(0);
        stack_pointer &= !0xF;

        for &arg_addr in argv.iter().rev() {
            if stack_pointer < core::mem::size_of::<u32>() {
                return Err(KernelError::Allocation);
            }
            stack_pointer -= core::mem::size_of::<*const i8>();
            stack[stack_pointer..stack_pointer + core::mem::size_of::<*const i8>()]
                .copy_from_slice(&arg_addr.to_ne_bytes());
        }

        let argv_ptr = USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END + stack_pointer;
        if stack_pointer < core::mem::size_of::<u32>() * 2 {
            return Err(KernelError::Allocation);
        }
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
            fd_table: Mutex::new(Self::default_fd_table()),
            children: Mutex::new(vec![]),
            parent: None,
            tasks: RwLock::new(None),
            filetype: ProcessFileType::Elf(elf),
            page_directory,
            stack: Page::new(USER_PROGRAM_STACK_SIZE)?,
            start_stack: USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START,
            entrypoint,
            heap: Mutex::new(BTreeMap::new()),
            cow_pages: Mutex::new(BTreeMap::new()),
        })
    }

    fn load_binary(filename: &str) -> Result<Self, KernelError> {
        let mut file = KERNEL
            .vfs
            .read()
            .open(filename)
            .map_err(|_| KernelError::Io)?;
        let stat = file.ops.stat().map_err(|_| KernelError::Io)?;

        let memory = Page::new(stat.size as usize).ok_or(KernelError::Allocation)?;

        file.ops
            .read(memory.as_mut_slice())
            .map_err(|_| KernelError::Io)?;
        let page_directory =
            PageDirectory::new_4gb(memory::PRESENT).ok_or(KernelError::Allocation)?;
        Ok(Self {
            pid: 0,
            fd_table: Mutex::new(Self::default_fd_table()),
            children: Mutex::new(vec![]),
            parent: None,
            tasks: RwLock::new(None),
            filetype: ProcessFileType::Binary(memory),
            page_directory,
            stack: Page::new(USER_PROGRAM_STACK_SIZE).ok_or(KernelError::Allocation)?,
            start_stack: USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START,
            entrypoint: PROGRAM_VIRTUAL_ADDRESS as u32,
            heap: Mutex::new(BTreeMap::new()),
            cow_pages: Mutex::new(BTreeMap::new()),
        })
    }

    fn default_fd_table() -> Vec<Option<ProcessDescriptor>> {
        let mut table = vec![None; FIRST_PROCESS_FD];

        for fd in 0..FIRST_PROCESS_FD {
            table[fd] = KERNEL
                .vfs
                .read()
                .open("/dev/console")
                .ok()
                .map(|handle| ProcessDescriptor::File(Arc::new(Mutex::new(handle))));
        }

        table
    }

    pub fn fork_from(pid: ProcessId, parent: &Process) -> Result<Self, KernelError> {
        let page_directory = parent
            .page_directory
            .cow_copy()
            .ok_or(KernelError::Allocation)?;

        let heap = parent
            .heap
            .lock()
            .iter()
            .map(|(addr, page)| (*addr, page.clone()))
            .collect();

        let cow_pages = parent
            .cow_pages
            .lock()
            .iter()
            .map(|(addr, page)| (*addr, page.clone()))
            .collect();

        let fd_table = parent
            .fd_table
            .lock()
            .iter()
            .map(|descriptor| descriptor.as_ref().map(|d| d.duplicate_for_fd_table()))
            .collect();

        Ok(Self {
            pid,
            fd_table: Mutex::new(fd_table),
            children: Mutex::new(vec![]),
            parent: Some(parent.pid),
            tasks: RwLock::new(None),
            filetype: parent.filetype.clone(),
            page_directory,
            stack: parent.stack.clone(),
            start_stack: parent.start_stack,
            entrypoint: parent.entrypoint,
            heap: Mutex::new(heap),
            cow_pages: Mutex::new(cow_pages),
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
                    if phdr.p_memsz == 0 {
                        continue;
                    }

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
            self.remove_cow_pages_in_range(addr, page.len() as u32);
        }
    }

    pub fn insert_fd(&self, descriptor: ProcessDescriptor) -> Result<i32, KernelError> {
        let mut table = self.fd_table.lock();

        for fd in FIRST_PROCESS_FD..table.len() {
            if table[fd].is_none() {
                table[fd] = Some(descriptor);
                return Ok(fd as i32);
            }
        }

        if table.len() >= MAX_PROCESS_FD {
            return Err(KernelError::Allocation);
        }

        table.push(Some(descriptor));
        Ok((table.len() - 1) as i32)
    }

    pub fn get_fd(&self, fd: i32) -> Option<ProcessDescriptor> {
        if fd < 0 {
            return None;
        }

        self.fd_table
            .lock()
            .get(fd as usize)
            .and_then(|descriptor| descriptor.clone())
    }

    pub fn remove_fd(&self, fd: i32) -> Option<ProcessDescriptor> {
        if fd < 0 {
            return None;
        }

        self.fd_table
            .lock()
            .get_mut(fd as usize)
            .and_then(|descriptor| descriptor.take())
    }

    pub fn close_descriptors(&self) {
        let mut table = self.fd_table.lock();
        for slot in table.iter_mut() {
            if let Some(descriptor) = slot.take() {
                descriptor.close();
            }
        }
        table.resize(FIRST_PROCESS_FD, None);
    }

    pub fn cleanup(&self) {
        for (addr, page) in self.heap.lock().iter() {
            let _ = self.page_directory.map_page(*addr, page, 0);
        }
        self.heap.lock().clear();

        for addr in self.cow_pages.lock().keys() {
            let _ = self.page_directory.set(*addr, 0);
        }
        self.cow_pages.lock().clear();
    }

    pub fn handle_cow_fault(&self, faulting_address: u32) -> Result<bool, KernelError> {
        let page_address = PageDirectory::align_address_down(faulting_address);
        let entry = self
            .page_directory
            .get(page_address)
            .map_err(|_| KernelError::Paging)?;

        if entry & memory::PRESENT == 0 || entry & memory::COW == 0 {
            return Ok(false);
        }

        let old_physical = entry & 0xFFFFF000;
        let new_page = Page::<u8>::new(PAGING_PAGE_SIZE).ok_or(KernelError::Allocation)?;
        let source =
            unsafe { core::slice::from_raw_parts(old_physical as *const u8, PAGING_PAGE_SIZE) };
        new_page.as_mut_slice()[..PAGING_PAGE_SIZE].copy_from_slice(source);

        let flags = (entry & 0xFFF | memory::WRITABLE) & !memory::COW;
        self.page_directory
            .map(page_address, new_page.as_ptr() as u32, flags)
            .map_err(|_| KernelError::Paging)?;

        self.cow_pages.lock().insert(page_address, new_page);

        Ok(true)
    }

    fn remove_cow_pages_in_range(&self, start: u32, size: u32) {
        let end = start.saturating_add(size);
        self.cow_pages
            .lock()
            .retain(|addr, _| *addr < start || *addr >= end);
    }
}
