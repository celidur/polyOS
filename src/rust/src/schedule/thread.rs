//! Thread lifetime policy; syscall decoding and userspace pthread bookkeeping live elsewhere.
use super::{
    process_manager::process_terminate,
    task::{WaitReason, task_next},
};
use crate::kernel::KERNEL;

pub fn exit_current(code: i32) -> ! {
    let context = KERNEL.with_task_manager(|tm| {
        let task = tm.get_current()?.read();
        Some((
            task.id,
            task.process.clone(),
            task.clear_child_tid,
            tm.process_task_count(task.process.pid) == 1,
        ))
    });
    if let Some((id, process, clear_tid, last)) = context {
        if clear_tid != 0 {
            let zero = 0_u32;
            let _ = super::task::copy_string_to_task(
                &process.page_directory,
                &zero as *const u32 as u32,
                clear_tid,
                4,
            );
            KERNEL.with_task_manager(|tm| {
                tm.wake_reason(
                    WaitReason::Futex {
                        pid: process.pid,
                        address: clear_tid,
                    },
                    usize::MAX,
                );
            });
        }
        if last {
            drop(process);
            process_terminate(code);
        } else {
            KERNEL.with_task_manager(|tm| tm.remove(id));
            if *process.main_task.read() == Some(id) {
                *process.main_task.write() =
                    KERNEL.with_task_manager(|tm| tm.first_process_task(process.pid));
            }
            drop(process);
        }
    }
    task_next()
}
