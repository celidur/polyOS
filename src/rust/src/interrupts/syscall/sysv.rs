use super::{abi, user};
use crate::{
    kernel::KERNEL,
    schedule::{
        ipc::{Operation, SEMAPHORES},
        task::task_next,
    },
};
pub fn syscall_ipc(call: u32, first: u32, second: u32, third: u32, ptr: u32, _fifth: u32) -> u32 {
    let Some(p) = KERNEL.with_task_manager(|tm| Some(tm.get_current()?.read().process.clone()))
    else {
        return abi::errno(abi::ESRCH);
    };
    let result: Result<Option<u32>, i32> = match call {
        2 => SEMAPHORES
            .lock()
            .get(first as i32, second, third, p.euid)
            .map(Some),
        1 => {
            if second == 0 || second > 64 {
                return abi::errno(abi::EINVAL);
            }
            let mut ops = vec![Operation::default(); second as usize];
            if user::copy_from_user(
                &p.page_directory,
                ptr,
                ops.as_mut_ptr() as *mut u8,
                second * 6,
            )
            .is_err()
            {
                return abi::errno(abi::EFAULT);
            }
            KERNEL
                .with_task_manager(|tm| SEMAPHORES.lock().operate(first, p.euid, &ops, tm))
                .map(|done| done.then_some(0))
        }
        3 => (|| {
            if third == 0 {
                return KERNEL
                    .with_task_manager(|tm| SEMAPHORES.lock().remove(first, p.euid, tm))
                    .map(|()| Some(0));
            }
            let mut argument = 0u32;
            if matches!(third, 13 | 16 | 17)
                && user::copy_from_user(
                    &p.page_directory,
                    ptr,
                    &mut argument as *mut u32 as *mut u8,
                    4,
                )
                .is_err()
            {
                return Err(14);
            }
            let mut values = SEMAPHORES
                .lock()
                .values(first, p.euid, matches!(third, 16 | 17))?;
            if third == 12 {
                return values
                    .get(second as usize)
                    .map(|v| Some(*v as u32))
                    .ok_or(22);
            }
            if third == 13 {
                if user::copy_to_user(
                    &p.page_directory,
                    argument,
                    values.as_ptr() as *const u8,
                    values.len() as u32 * 2,
                )
                .is_err()
                {
                    return Err(14);
                }
                return Ok(Some(0));
            }
            if third == 16 {
                let Some(v) = values.get_mut(second as usize) else {
                    return Err(22);
                };
                if argument > 32767 {
                    return Err(34);
                }
                *v = argument as u16;
            } else if third == 17 {
                if user::copy_from_user(
                    &p.page_directory,
                    argument,
                    values.as_mut_ptr() as *mut u8,
                    values.len() as u32 * 2,
                )
                .is_err()
                {
                    return Err(14);
                }
            } else {
                return Err(95);
            }
            KERNEL
                .with_task_manager(|tm| SEMAPHORES.lock().replace(first, p.euid, &values, tm))
                .map(|()| Some(0))
        })(),
        _ => Err(38),
    };
    match result {
        Ok(Some(n)) => n,
        Ok(None) => {
            drop(p);
            task_next()
        }
        Err(e) => abi::errno(e),
    }
}
