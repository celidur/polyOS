use super::{abi, user};
use crate::{kernel::KERNEL, memory::address_space::VmError};
#[repr(C)]
#[derive(Default)]
struct MmapArgs {
    addr: u32,
    len: u32,
    prot: u32,
    flags: u32,
    fd: i32,
    offset: u32,
}
fn error(e: VmError) -> u32 {
    abi::errno(match e {
        VmError::Invalid => abi::EINVAL,
        VmError::NoMemory => abi::ENOMEM,
        VmError::Fault => abi::EFAULT,
        VmError::Unsupported => abi::ENOTSUP,
        VmError::BadFd => abi::EBADF,
        VmError::Io => abi::EIO,
        VmError::Access => abi::EACCES,
    })
}
pub fn syscall_mmap(ptr: u32) -> u32 {
    let Some(p) = KERNEL.with_task_manager(|tm| Some(tm.get_current()?.read().process.clone()))
    else {
        return abi::errno(abi::ESRCH);
    };
    let mut a = MmapArgs::default();
    if user::copy_from_user(&p.page_directory, ptr, &mut a as *mut _ as *mut u8, 24).is_err() {
        return abi::errno(abi::EFAULT);
    }
    let file = if a.flags & 0x20 != 0 {
        if a.offset != 0 {
            return abi::errno(abi::EINVAL);
        }
        None
    } else {
        if p.get_status_flags(a.fd).is_some_and(|f| f & 3 == 1) {
            return error(VmError::Access);
        }
        match p.get_fd(a.fd) {
            Some(d) => Some(d),
            None => return abi::errno(abi::EBADF),
        }
    };
    let result = p
        .mappings
        .lock()
        .map(&p, a.addr, a.len, a.prot, a.flags, file, a.offset);
    match result {
        Ok(a) => a,
        Err(e) => error(e),
    }
}
pub fn syscall_munmap(addr: u32, len: u32) -> u32 {
    let Some(p) = KERNEL.with_task_manager(|tm| Some(tm.get_current()?.read().process.clone()))
    else {
        return abi::errno(abi::ESRCH);
    };
    let result = p.mappings.lock().unmap(&p, addr, len);
    match result {
        Ok(()) => 0,
        Err(e) => error(e),
    }
}
pub fn syscall_mprotect(addr: u32, len: u32, prot: u32) -> u32 {
    let Some(p) = KERNEL.with_task_manager(|tm| Some(tm.get_current()?.read().process.clone()))
    else {
        return abi::errno(abi::ESRCH);
    };
    let result = p
        .mappings
        .lock()
        .protect(&p.page_directory, addr, len, prot);
    match result {
        Ok(()) => 0,
        Err(e) => error(e),
    }
}
