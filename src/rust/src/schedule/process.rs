use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use spin::{Mutex, RwLock};

use crate::{
    constant::{
        PAGING_PAGE_SIZE, PROGRAM_VIRTUAL_ADDRESS, USER_HEAP_END, USER_HEAP_START,
        USER_PROGRAM_STACK_SIZE, USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END,
        USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START,
    },
    error::KernelError,
    fs::{FileHandle, FileMetadata, FsError, Pipe, PipeEnd, PipeError},
    kernel::KERNEL,
    memory::{self, Page, PageDirectory},
    schedule::loader::elf::{ElfFile, PF_W},
};

use super::task::{Registers, TaskId};

pub type ProcessId = u32;
const FIRST_PROCESS_FD: usize = 3;
const MAX_PROCESS_FD: usize = 128;
pub const MAX_SIGNAL: usize = 31;
pub const SIG_DFL: u32 = 0;
pub const SIG_IGN: u32 = 1;
pub const SIGKILL: u32 = 9;
pub const SIGCHLD: u32 = 17;
pub const SIGCONT: u32 = 18;
pub const SIGSTOP: u32 = 19;
pub const SIGNAL_FRAME_MAGIC: u32 = 0x5349_4731;
pub const FD_CLOEXEC: u32 = 1;
const O_ACCMODE: u32 = 0x3;
pub const O_APPEND: u32 = 0x400;
pub const O_NONBLOCK: u32 = 0x800;
const OPEN_STATUS_FLAGS: u32 = O_ACCMODE | O_APPEND | O_NONBLOCK;
const SETTABLE_STATUS_FLAGS: u32 = O_APPEND | O_NONBLOCK;

#[derive(Clone)]
pub enum ProcessDescriptor {
    File(Arc<Mutex<FileHandle>>),
    Directory(Arc<Mutex<DirectoryHandle>>),
    Socket(Arc<Mutex<SocketHandle>>),
    Pipe {
        pipe: Arc<Mutex<Pipe>>,
        end: PipeEnd,
    },
}

#[derive(Clone)]
pub struct ProcessFd {
    descriptor: ProcessDescriptor,
    flags: u32,
    status_flags: u32,
}

impl ProcessFd {
    fn new(descriptor: ProcessDescriptor) -> Self {
        Self::new_with_status_flags(descriptor, 0)
    }

    fn new_with_status_flags(descriptor: ProcessDescriptor, status_flags: u32) -> Self {
        Self {
            descriptor,
            flags: 0,
            status_flags: status_flags & OPEN_STATUS_FLAGS,
        }
    }

    fn duplicate_for_fork(&self) -> Self {
        Self {
            descriptor: self.descriptor.duplicate_for_fd_table(),
            flags: self.flags,
            status_flags: self.status_flags,
        }
    }

    fn duplicate_for_dup(&self) -> Self {
        Self {
            descriptor: self.descriptor.duplicate_for_fd_table(),
            flags: 0,
            status_flags: self.status_flags,
        }
    }

    fn close(self) {
        self.descriptor.close();
    }
}

#[derive(Clone)]
pub struct DirectoryHandle {
    pub path: String,
    pub entries: Vec<String>,
    pub offset: usize,
    pub metadata: FileMetadata,
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
            Self::Directory(_) => Err(FsError::IsADirectory),
            Self::Pipe { pipe, end } => pipe.lock().read(*end, buf).map_err(pipe_error),
            Self::Socket(_) => Err(FsError::Unsupported),
        }
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, FsError> {
        match self {
            Self::File(file) => file.lock().ops.write(buf),
            Self::Directory(_) => Err(FsError::IsADirectory),
            Self::Pipe { pipe, end } => pipe.lock().write(*end, buf).map_err(pipe_error),
            Self::Socket(_) => Err(FsError::Unsupported),
        }
    }

    pub fn seek(&self, pos: usize) -> Result<usize, FsError> {
        match self {
            Self::File(file) => file.lock().ops.seek(pos),
            Self::Directory(directory) => {
                let mut directory = directory.lock();
                directory.offset = pos.min(directory.entries.len());
                Ok(directory.offset)
            }
            _ => Err(FsError::Unsupported),
        }
    }

    pub fn ioctl(&self, request: u32, arg: u32, directory: &PageDirectory) -> Result<u32, FsError> {
        match self {
            Self::File(file) => file.lock().ops.ioctl(request, arg, directory),
            Self::Directory(_) => Err(FsError::Unsupported),
            _ => Err(FsError::Unsupported),
        }
    }

    pub fn stat(&self) -> Result<FileMetadata, FsError> {
        match self {
            Self::File(file) => file.lock().ops.stat(),
            Self::Directory(directory) => Ok(directory.lock().metadata.clone()),
            _ => Err(FsError::Unsupported),
        }
    }

    pub fn duplicate_for_fd_table(&self) -> Self {
        match self {
            Self::File(file) => Self::File(file.clone()),
            Self::Directory(directory) => Self::Directory(directory.clone()),
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
            Self::Directory(_) => {}
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
    pub env: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Zombie { status: i32 },
}

#[derive(Clone, Copy, Default)]
pub struct SignalAction {
    pub handler: u32,
    pub flags: u32,
    pub restorer: u32,
    pub mask: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SignalFrame {
    pub magic: u32,
    pub registers: Registers,
}

pub struct Process {
    pub pid: ProcessId,
    pub fd_table: Mutex<Vec<Option<ProcessFd>>>,
    pub children: Mutex<Vec<ProcessId>>,
    parent: Mutex<Option<ProcessId>>,
    state: Mutex<ProcessState>,
    pub entrypoint: u32,
    pub tasks: RwLock<Option<TaskId>>,
    pub cwd: Mutex<String>,
    pub env: Mutex<Vec<String>>,
    pub filetype: ProcessFileType,
    pub page_directory: PageDirectory,
    pub stack: Page<u8>,
    pub start_stack: usize,
    pub brk: Mutex<u32>,
    signal_actions: Mutex<[SignalAction; MAX_SIGNAL + 1]>,
    brk_pages: Mutex<BTreeMap<u32, Page<u8>>>,
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
        process.set_parent(parent);

        process.map_memory()?;

        let args = args.unwrap_or_else(|| ProcessArguments {
            args: vec![filename.to_string()],
            env: default_environment(),
        });

        serial_println!("Process {} args: {:?}", pid, args.args);

        let stack = process.stack.as_mut_slice();
        let mut stack_pointer = stack.len();

        let argv = push_stack_strings(stack, &mut stack_pointer, &args.args)?;
        let envp = push_stack_strings(stack, &mut stack_pointer, &args.env)?;
        stack_pointer &= !0xF;

        let envp_ptr = push_stack_pointer_array(stack, &mut stack_pointer, &envp)?;
        let argv_ptr = push_stack_pointer_array(stack, &mut stack_pointer, &argv)?;

        push_stack_u32(stack, &mut stack_pointer, envp_ptr as u32)?;
        push_stack_u32(stack, &mut stack_pointer, argv_ptr as u32)?;
        push_stack_u32(stack, &mut stack_pointer, args.args.len() as u32)?;

        process.env = Mutex::new(args.env);

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
            parent: Mutex::new(None),
            state: Mutex::new(ProcessState::Running),
            tasks: RwLock::new(None),
            filetype: ProcessFileType::Elf(elf),
            page_directory,
            stack: Page::new(USER_PROGRAM_STACK_SIZE)?,
            start_stack: USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START,
            entrypoint,
            brk: Mutex::new(USER_HEAP_START as u32),
            signal_actions: Mutex::new([SignalAction::default(); MAX_SIGNAL + 1]),
            brk_pages: Mutex::new(BTreeMap::new()),
            cow_pages: Mutex::new(BTreeMap::new()),
            cwd: Mutex::new("/".to_string()),
            env: Mutex::new(default_environment()),
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
            parent: Mutex::new(None),
            state: Mutex::new(ProcessState::Running),
            tasks: RwLock::new(None),
            filetype: ProcessFileType::Binary(memory),
            page_directory,
            stack: Page::new(USER_PROGRAM_STACK_SIZE).ok_or(KernelError::Allocation)?,
            start_stack: USER_PROGRAM_VIRTUAL_STACK_ADDRESS_START,
            entrypoint: PROGRAM_VIRTUAL_ADDRESS as u32,
            brk: Mutex::new(USER_HEAP_START as u32),
            signal_actions: Mutex::new([SignalAction::default(); MAX_SIGNAL + 1]),
            brk_pages: Mutex::new(BTreeMap::new()),
            cow_pages: Mutex::new(BTreeMap::new()),
            cwd: Mutex::new("/".to_string()),
            env: Mutex::new(default_environment()),
        })
    }

    fn default_fd_table() -> Vec<Option<ProcessFd>> {
        let mut table = vec![None; FIRST_PROCESS_FD];

        for fd in 0..FIRST_PROCESS_FD {
            table[fd] = KERNEL.vfs.read().open("/dev/console").ok().map(|handle| {
                ProcessFd::new(ProcessDescriptor::File(Arc::new(Mutex::new(handle))))
            });
        }

        table
    }

    pub fn fork_from(pid: ProcessId, parent: &Process) -> Result<Self, KernelError> {
        let page_directory = parent
            .page_directory
            .cow_copy()
            .ok_or(KernelError::Allocation)?;

        let cow_pages = parent
            .cow_pages
            .lock()
            .iter()
            .map(|(addr, page)| (*addr, page.clone()))
            .collect();

        let brk_pages = parent
            .brk_pages
            .lock()
            .iter()
            .map(|(addr, page)| (*addr, page.clone()))
            .collect();

        let fd_table = parent
            .fd_table
            .lock()
            .iter()
            .map(|descriptor| descriptor.as_ref().map(|d| d.duplicate_for_fork()))
            .collect();
        let signal_actions = *parent.signal_actions.lock();

        Ok(Self {
            pid,
            fd_table: Mutex::new(fd_table),
            children: Mutex::new(vec![]),
            parent: Mutex::new(Some(parent.pid)),
            state: Mutex::new(ProcessState::Running),
            tasks: RwLock::new(None),
            filetype: parent.filetype.clone(),
            page_directory,
            stack: parent.stack.clone(),
            start_stack: parent.start_stack,
            entrypoint: parent.entrypoint,
            brk: Mutex::new(*parent.brk.lock()),
            signal_actions: Mutex::new(signal_actions),
            brk_pages: Mutex::new(brk_pages),
            cow_pages: Mutex::new(cow_pages),
            cwd: Mutex::new(parent.cwd.lock().clone()),
            env: Mutex::new(parent.env.lock().clone()),
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
                for segment in elf.segments() {
                    let mut flags = memory::PRESENT | memory::USER_ACCESS;
                    if (segment.flags() & PF_W) != 0 {
                        flags |= memory::WRITABLE;
                    }
                    self.page_directory
                        .map_page(segment.virtual_address(), segment.memory(), flags)
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

    pub fn set_program_break(&self, requested_break: u32) -> u32 {
        let mut current_break = self.brk.lock();
        if requested_break == 0 {
            return *current_break;
        }

        if !(USER_HEAP_START as u32..=USER_HEAP_END as u32).contains(&requested_break) {
            return *current_break;
        }

        let old_mapped_end = align_up(*current_break);
        let new_mapped_end = align_up(requested_break);

        if new_mapped_end > old_mapped_end {
            let mut mapped = Vec::new();
            let mut addr = old_mapped_end;
            while addr < new_mapped_end {
                let Some(page) = Page::<u8>::new(PAGING_PAGE_SIZE) else {
                    self.unmap_brk_pages(&mapped);
                    return *current_break;
                };

                if self
                    .page_directory
                    .map_page(
                        addr,
                        &page,
                        memory::PRESENT | memory::WRITABLE | memory::USER_ACCESS,
                    )
                    .is_err()
                {
                    self.unmap_brk_pages(&mapped);
                    return *current_break;
                }

                self.brk_pages.lock().insert(addr, page);
                mapped.push(addr);
                addr = addr.saturating_add(PAGING_PAGE_SIZE as u32);
            }
        } else if new_mapped_end < old_mapped_end {
            let mut addr = new_mapped_end;
            while addr < old_mapped_end {
                if self.brk_pages.lock().remove(&addr).is_some() {
                    let _ = self.page_directory.set(addr, 0);
                    self.remove_cow_pages_in_range(addr, PAGING_PAGE_SIZE as u32);
                }
                addr = addr.saturating_add(PAGING_PAGE_SIZE as u32);
            }
        }

        *current_break = requested_break;
        *current_break
    }

    fn unmap_brk_pages(&self, pages: &[u32]) {
        let mut brk_pages = self.brk_pages.lock();
        for &addr in pages {
            brk_pages.remove(&addr);
            let _ = self.page_directory.set(addr, 0);
            self.remove_cow_pages_in_range(addr, PAGING_PAGE_SIZE as u32);
        }
    }

    pub fn insert_fd(&self, descriptor: ProcessDescriptor) -> Result<i32, KernelError> {
        self.insert_fd_with_status_flags(descriptor, 0)
    }

    pub fn insert_fd_with_status_flags(
        &self,
        descriptor: ProcessDescriptor,
        status_flags: u32,
    ) -> Result<i32, KernelError> {
        self.insert_fd_from(
            FIRST_PROCESS_FD,
            ProcessFd::new_with_status_flags(descriptor, status_flags),
        )
    }

    fn insert_fd_from(&self, min_fd: usize, fd_entry: ProcessFd) -> Result<i32, KernelError> {
        if min_fd >= MAX_PROCESS_FD {
            return Err(KernelError::Allocation);
        }

        let mut table = self.fd_table.lock();

        for fd in min_fd..table.len() {
            if table[fd].is_none() {
                table[fd] = Some(fd_entry);
                return Ok(fd as i32);
            }
        }

        if table.len() >= MAX_PROCESS_FD {
            return Err(KernelError::Allocation);
        }

        while table.len() < min_fd {
            table.push(None);
        }

        table.push(Some(fd_entry));
        Ok((table.len() - 1) as i32)
    }

    pub fn duplicate_fd(&self, fd: i32) -> Result<i32, KernelError> {
        self.duplicate_fd_from(fd, 0)
    }

    pub fn duplicate_fd_from(&self, fd: i32, min_fd: i32) -> Result<i32, KernelError> {
        if min_fd < 0 {
            return Err(KernelError::Io);
        }

        let fd_entry = self.get_fd_entry(fd).ok_or(KernelError::Io)?;
        self.insert_fd_from(min_fd as usize, fd_entry.duplicate_for_dup())
    }

    pub fn duplicate_fd_to(&self, old_fd: i32, new_fd: i32) -> Result<i32, KernelError> {
        if old_fd < 0 || new_fd < 0 || new_fd as usize >= MAX_PROCESS_FD {
            return Err(KernelError::Io);
        }

        let fd_entry = self.get_fd_entry(old_fd).ok_or(KernelError::Io)?;
        if old_fd == new_fd {
            return Ok(new_fd);
        }

        let fd_entry = fd_entry.duplicate_for_dup();
        let old_descriptor = {
            let mut table = self.fd_table.lock();
            if table.len() <= new_fd as usize {
                table.resize_with(new_fd as usize + 1, || None);
            }
            core::mem::replace(&mut table[new_fd as usize], Some(fd_entry))
        };

        if let Some(old_descriptor) = old_descriptor {
            old_descriptor.close();
        }

        Ok(new_fd)
    }

    pub fn get_fd(&self, fd: i32) -> Option<ProcessDescriptor> {
        self.get_fd_entry(fd).map(|fd_entry| fd_entry.descriptor)
    }

    fn get_fd_entry(&self, fd: i32) -> Option<ProcessFd> {
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
            .map(|fd| fd.descriptor)
    }

    pub fn get_fd_flags(&self, fd: i32) -> Option<u32> {
        if fd < 0 {
            return None;
        }

        self.fd_table
            .lock()
            .get(fd as usize)
            .and_then(|fd| fd.as_ref())
            .map(|fd| fd.flags)
    }

    pub fn set_fd_flags(&self, fd: i32, flags: u32) -> Result<(), KernelError> {
        if fd < 0 {
            return Err(KernelError::Io);
        }

        let mut table = self.fd_table.lock();
        let Some(Some(fd_entry)) = table.get_mut(fd as usize) else {
            return Err(KernelError::Io);
        };

        fd_entry.flags = flags & FD_CLOEXEC;
        Ok(())
    }

    pub fn get_status_flags(&self, fd: i32) -> Option<u32> {
        if fd < 0 {
            return None;
        }

        self.fd_table
            .lock()
            .get(fd as usize)
            .and_then(|fd| fd.as_ref())
            .map(|fd| fd.status_flags)
    }

    pub fn set_status_flags(&self, fd: i32, flags: u32) -> Result<(), KernelError> {
        if fd < 0 {
            return Err(KernelError::Io);
        }

        let mut table = self.fd_table.lock();
        let Some(Some(fd_entry)) = table.get_mut(fd as usize) else {
            return Err(KernelError::Io);
        };

        fd_entry.status_flags =
            (fd_entry.status_flags & O_ACCMODE) | (flags & SETTABLE_STATUS_FLAGS);
        Ok(())
    }

    pub fn take_exec_fd_table(&self) -> Vec<Option<ProcessFd>> {
        let mut old_table = self.fd_table.lock();
        let table = core::mem::take(&mut *old_table);
        table
            .into_iter()
            .map(|entry| match entry {
                Some(fd) if fd.flags & FD_CLOEXEC != 0 => {
                    fd.close();
                    None
                }
                entry => entry,
            })
            .collect()
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
        for addr in self.brk_pages.lock().keys() {
            let _ = self.page_directory.set(*addr, 0);
        }
        self.brk_pages.lock().clear();

        for addr in self.cow_pages.lock().keys() {
            let _ = self.page_directory.set(*addr, 0);
        }
        self.cow_pages.lock().clear();
    }

    pub fn resolve_path(&self, path: &str) -> Option<String> {
        if path.is_empty() {
            return None;
        }

        let mut components = Vec::new();
        if !path.starts_with('/') {
            for component in self.cwd.lock().split('/') {
                if !component.is_empty() {
                    components.push(component.to_string());
                }
            }
        }

        for component in path.split('/') {
            match component {
                "" | "." => {}
                ".." => {
                    components.pop();
                }
                component => components.push(component.to_string()),
            }
        }

        let mut resolved = String::from("/");
        for (index, component) in components.iter().enumerate() {
            if index != 0 {
                resolved.push('/');
            }
            resolved.push_str(component);
        }

        Some(resolved)
    }

    pub fn set_cwd(&self, path: String) {
        *self.cwd.lock() = path;
    }

    pub fn parent_pid(&self) -> Option<ProcessId> {
        *self.parent.lock()
    }

    pub fn set_parent(&self, parent: Option<ProcessId>) {
        *self.parent.lock() = parent;
    }

    pub fn mark_zombie(&self, status: i32) {
        *self.state.lock() = ProcessState::Zombie { status };
    }

    pub fn zombie_status(&self) -> Option<i32> {
        match *self.state.lock() {
            ProcessState::Zombie { status } => Some(status),
            ProcessState::Running => None,
        }
    }

    pub fn get_signal_action(&self, signal: u32) -> Option<SignalAction> {
        if !valid_signal(signal) {
            return None;
        }

        Some(self.signal_actions.lock()[signal as usize])
    }

    pub fn set_signal_action(&self, signal: u32, action: SignalAction) -> Option<SignalAction> {
        if !valid_signal(signal) {
            return None;
        }

        let mut actions = self.signal_actions.lock();
        let old = actions[signal as usize];
        actions[signal as usize] = action;
        Some(old)
    }

    pub fn signal_actions_for_exec(&self) -> [SignalAction; MAX_SIGNAL + 1] {
        let mut exec_actions = [SignalAction::default(); MAX_SIGNAL + 1];
        for (signal, action) in self.signal_actions.lock().iter().enumerate() {
            if signal != 0 && action.handler == SIG_IGN {
                exec_actions[signal] = *action;
            }
        }
        exec_actions
    }

    pub fn replace_signal_actions(&self, actions: [SignalAction; MAX_SIGNAL + 1]) {
        *self.signal_actions.lock() = actions;
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

pub fn valid_signal(signal: u32) -> bool {
    signal > 0 && signal as usize <= MAX_SIGNAL
}

pub fn signal_default_ignored(signal: u32) -> bool {
    signal == SIGCHLD || signal == SIGCONT
}

fn align_up(address: u32) -> u32 {
    (address + PAGING_PAGE_SIZE as u32 - 1) & !(PAGING_PAGE_SIZE as u32 - 1)
}

fn default_environment() -> Vec<String> {
    vec!["PATH=/bin".to_string()]
}

fn push_stack_strings(
    stack: &mut [u8],
    stack_pointer: &mut usize,
    values: &[String],
) -> Result<Vec<usize>, KernelError> {
    let mut pointers = Vec::with_capacity(values.len());
    for value in values.iter().rev() {
        let bytes = value.as_bytes();
        if *stack_pointer < bytes.len() + 1 {
            return Err(KernelError::Allocation);
        }

        *stack_pointer -= bytes.len() + 1;
        stack[*stack_pointer..*stack_pointer + bytes.len()].copy_from_slice(bytes);
        stack[*stack_pointer + bytes.len()] = 0;
        pointers.push(USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END + *stack_pointer);
    }

    pointers.reverse();
    Ok(pointers)
}

fn push_stack_pointer_array(
    stack: &mut [u8],
    stack_pointer: &mut usize,
    pointers: &[usize],
) -> Result<usize, KernelError> {
    push_stack_u32(stack, stack_pointer, 0)?;
    for &pointer in pointers.iter().rev() {
        push_stack_u32(stack, stack_pointer, pointer as u32)?;
    }

    Ok(USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END + *stack_pointer)
}

fn push_stack_u32(
    stack: &mut [u8],
    stack_pointer: &mut usize,
    value: u32,
) -> Result<(), KernelError> {
    if *stack_pointer < core::mem::size_of::<u32>() {
        return Err(KernelError::Allocation);
    }

    *stack_pointer -= core::mem::size_of::<u32>();
    stack[*stack_pointer..*stack_pointer + core::mem::size_of::<u32>()]
        .copy_from_slice(&value.to_ne_bytes());
    Ok(())
}

fn pipe_error(error: PipeError) -> FsError {
    match error {
        PipeError::WouldBlock => FsError::WouldBlock,
        PipeError::BrokenPipe => FsError::BrokenPipe,
        PipeError::WrongEnd => FsError::InvalidArgument,
    }
}
