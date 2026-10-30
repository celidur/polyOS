//! getrandom: hardware randomness when present, explicit insecure hash seeding otherwise.
use super::{abi, user};
use crate::kernel::KERNEL;
use core::sync::atomic::{AtomicU32, Ordering};
static SEQUENCE: AtomicU32 = AtomicU32::new(0x9e3779b9);
pub fn syscall_getrandom(ptr: u32, length: u32, flags: u32) -> u32 {
    if flags & !7 != 0 || flags & 6 == 6 {
        return abi::errno(abi::EINVAL);
    }
    let Some(p) = KERNEL.with_task_manager(|tm| Some(tm.get_current()?.read().process.clone()))
    else {
        return abi::errno(abi::ESRCH);
    };
    let count = length.min(65536);
    if user::validate_range(&p, ptr, count, true).is_err() {
        return abi::errno(abi::EFAULT);
    }
    let hardware = core::arch::x86::__cpuid(1).ecx & (1 << 30) != 0;
    if count != 0 && !hardware && flags & 4 == 0 {
        return abi::errno(abi::EAGAIN);
    }
    let mut bytes = vec![0u8; count as usize];
    for chunk in bytes.chunks_mut(4) {
        let mut value = 0;
        let mut ready = false;
        if hardware {
            for _ in 0..10 {
                if unsafe { core::arch::x86::_rdrand32_step(&mut value) } == 1 {
                    ready = true;
                    break;
                }
            }
        }
        if !ready {
            if flags & 4 == 0 {
                return abi::errno(abi::EAGAIN);
            }
            // GRND_INSECURE is explicitly non-cryptographic. Never expose this
            // path as secure entropy or silently satisfy a flags=0 request.
            value = SEQUENCE.fetch_add(0x9e3779b9, Ordering::Relaxed)
                ^ unsafe { core::arch::x86::_rdtsc() } as u32;
            value ^= value >> 16;
            value = value.wrapping_mul(0x7feb352d);
            value ^= value >> 15;
        }
        chunk.copy_from_slice(&value.to_ne_bytes()[..chunk.len()]);
    }
    if user::copy_to_user(&p.page_directory, ptr, bytes.as_ptr(), count).is_err() {
        return abi::errno(abi::EFAULT);
    }
    count
}
