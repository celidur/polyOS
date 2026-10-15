use super::{abi, user};
use crate::{inspection, kernel::KERNEL};
#[repr(C)]
#[derive(Default)]
struct SysInfo {
    uptime: i32,
    loads: [u32; 3],
    totalram: u32,
    freeram: u32,
    sharedram: u32,
    bufferram: u32,
    totalswap: u32,
    freeswap: u32,
    procs: u16,
    pad: u16,
    totalhigh: u32,
    freehigh: u32,
    mem_unit: u32,
    reserved: [u8; 8],
}
const _: [(); 64] = [(); core::mem::size_of::<SysInfo>()];
pub fn syscall_sysinfo(ptr: u32) -> u32 {
    let Some(p) = KERNEL.with_task_manager(|tm| Some(tm.get_current()?.read().process.clone()))
    else {
        return abi::errno(abi::ESRCH);
    };
    let s = inspection::snapshot();
    let info = SysInfo {
        uptime: s.uptime() as i32,
        totalram: s.total,
        freeram: s.free,
        procs: s.processes,
        mem_unit: 1,
        ..Default::default()
    };
    if user::write_value(&p.page_directory, ptr, &info).is_err() {
        abi::errno(abi::EFAULT)
    } else {
        0
    }
}
