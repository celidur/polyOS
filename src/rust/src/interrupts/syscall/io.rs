use crate::{
    constant::TIMER_HZ,
    kernel::KERNEL,
    schedule::task::{task_current_set_return_value, task_next},
};

use super::{abi, user};

const CLOCK_REALTIME: u32 = 0;
const CLOCK_MONOTONIC: u32 = 1;
const NSEC_PER_SEC: u64 = 1_000_000_000;
const USEC_PER_SEC: u64 = 1_000_000;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct TimeSpec {
    tv_sec: i32,
    tv_nsec: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct TimeVal {
    tv_sec: i32,
    tv_usec: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct TimeZone {
    tz_minuteswest: i32,
    tz_dsttime: i32,
}

pub fn syscall_nanosleep(req_ptr: u32, rem_ptr: u32) -> u32 {
    let Some((process, req_ptr)) = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some((task.process.clone(), req_ptr))
    }) else {
        return abi::errno(abi::EFAULT);
    };

    if req_ptr == 0 {
        return abi::errno(abi::EFAULT);
    }

    let mut requested = TimeSpec::default();
    if user::copy_from_user(
        &process.page_directory,
        req_ptr,
        &mut requested as *mut TimeSpec as *mut u8,
        core::mem::size_of::<TimeSpec>() as u32,
    )
    .is_err()
    {
        return abi::errno(abi::EFAULT);
    }

    if requested.tv_sec < 0 || requested.tv_nsec < 0 || requested.tv_nsec as u64 >= NSEC_PER_SEC {
        return abi::errno(abi::EINVAL);
    }

    let sleep_ticks = (requested.tv_sec as u64)
        .saturating_mul(TIMER_HZ as u64)
        .saturating_add(
            (requested.tv_nsec as u64)
                .saturating_mul(TIMER_HZ as u64)
                .saturating_add(NSEC_PER_SEC - 1)
                / NSEC_PER_SEC,
        );

    if sleep_ticks == 0 {
        return 0;
    }

    if rem_ptr != 0 && user::validate_range(&process, rem_ptr, 8, true).is_err() {
        return abi::errno(abi::EFAULT);
    }
    let sleep_set = KERNEL.with_task_manager(|tm| {
        let now = tm.get_tick();
        if let Some(t) = tm.get_current() {
            t.write().sleep_remaining = if rem_ptr != 0 {
                Some((rem_ptr, now.saturating_add(sleep_ticks)))
            } else {
                None
            };
        }
        tm.sleep_current_until(now.saturating_add(sleep_ticks))
            .is_ok()
    });

    if !sleep_set {
        return abi::errno(abi::EINVAL);
    }

    drop(process);
    task_current_set_return_value(0);
    task_next();
}

pub fn syscall_gettimeofday(tv_ptr: u32, tz_ptr: u32) -> u32 {
    let Some((process, tv_ptr, tz_ptr)) = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some((task.process.clone(), tv_ptr, tz_ptr))
    }) else {
        return abi::errno(abi::EFAULT);
    };

    let ticks = KERNEL.with_task_manager(|tm| tm.get_tick());
    if tv_ptr != 0 {
        let tv = TimeVal {
            tv_sec: (ticks / TIMER_HZ as u64) as i32,
            tv_usec: ((ticks % TIMER_HZ as u64) * USEC_PER_SEC / TIMER_HZ as u64) as i32,
        };
        if user::write_value(&process.page_directory, tv_ptr, &tv).is_err() {
            return abi::errno(abi::EFAULT);
        }
    }

    if tz_ptr != 0 {
        let tz = TimeZone::default();
        if user::write_value(&process.page_directory, tz_ptr, &tz).is_err() {
            return abi::errno(abi::EFAULT);
        }
    }

    0
}

pub fn syscall_clock_gettime(clockid: u32, tp_ptr: u32) -> u32 {
    let Some((process, clock_id, tp_ptr)) = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some((task.process.clone(), clockid, tp_ptr))
    }) else {
        return abi::errno(abi::EFAULT);
    };

    if clock_id != CLOCK_REALTIME && clock_id != CLOCK_MONOTONIC {
        return abi::errno(abi::EINVAL);
    }
    if tp_ptr == 0 {
        return abi::errno(abi::EFAULT);
    }

    let ticks = KERNEL.with_task_manager(|tm| tm.get_tick());
    let tp = TimeSpec {
        tv_sec: (ticks / TIMER_HZ as u64) as i32,
        tv_nsec: ((ticks % TIMER_HZ as u64) * NSEC_PER_SEC / TIMER_HZ as u64) as i32,
    };

    if user::write_value(&process.page_directory, tp_ptr, &tp).is_err() {
        return abi::errno(abi::EFAULT);
    }

    0
}

pub fn syscall_ioctl(fd: u32, request: u32, arg: u32) -> u32 {
    let Some((process, fd, request, arg)) = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some((task.process.clone(), fd as i32, request, arg))
    }) else {
        return abi::errno(abi::EFAULT);
    };

    match process.get_fd(fd) {
        Some(descriptor) => match descriptor.ioctl(request, arg, &process.page_directory) {
            Ok(result) => result,
            Err(crate::fs::FsError::Unsupported) => abi::errno(abi::ENOTTY),
            Err(error) => super::file::fs_errno(error),
        },
        _ => abi::errno(abi::EBADF),
    }
}
