use alloc::string::ToString;

use crate::{
    constant::MAX_PATH,
    interrupts::InterruptFrame,
    kernel::KERNEL,
    schedule::{
        process::{ProcessArguments, command_argument},
        process_manager::process_terminate,
        task::{copy_string_from_task, task_next},
    },
};

pub fn syscall_process_load_start(_frame: &InterruptFrame) -> u32 {
    let res = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;

        let file_user_ptr = current_task.read().get_stack_item(0);
        if file_user_ptr == 0 {
            return None;
        }

        let mut filename: [u8; MAX_PATH] = [0; MAX_PATH];

        if copy_string_from_task(
            &current_task.read().process.page_directory,
            file_user_ptr,
            filename.as_mut_ptr() as u32,
            filename.len() as u32,
        )
        .is_err()
        {
            return None;
        }

        let filename = unsafe { core::ffi::CStr::from_ptr(filename.as_ptr() as *const i8) };
        let filename = filename.to_str().unwrap_or("");
        Some((filename, current_task.read().process.pid))
    });

    if res.is_none() {
        let res = u32::MAX;
        return res;
    }
    let (program_name, pid) = res.unwrap();

    let _ = KERNEL.with_process_manager(|pm| pm.spawn(program_name, Some(pid), None));

    task_next();

    0
}

pub fn syscall_exec(_frame: &InterruptFrame) -> u32 {
    let res = KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;

        let ptr = current_task
            .read()
            .virtual_address_to_physical(current_task.read().get_stack_item(0))
            .unwrap_or(0) as *mut command_argument;
        if ptr.is_null() {
            return None;
        }

        let mut args = vec![];

        let mut command = unsafe { &*ptr };
        let str = unsafe {
            core::ffi::CStr::from_ptr(command.argument.as_ptr())
                .to_str()
                .unwrap_or("")
        };
        args.push(str.to_string());
        while !command.next.is_null() {
            command = unsafe { &*(command.next as *const command_argument) };
            let str = unsafe {
                core::ffi::CStr::from_ptr(command.argument.as_ptr())
                    .to_str()
                    .unwrap_or("")
            };
            args.push(str.to_string());
        }

        Some((args, current_task.read().process.pid))
    });

    if res.is_none() {
        let res = u32::MAX;
        return res;
    }
    let (args, pid) = res.unwrap();
    let program_name = args[0].clone();

    let args = ProcessArguments { args };

    let res =
        KERNEL.with_process_manager(|pm| pm.spawn(program_name.as_str(), Some(pid), Some(args)));

    if res.is_err() {
        let res = u32::MAX;
        return res;
    }

    task_next();

    0
}

pub fn syscall_exit(_frame: &InterruptFrame) -> u32 {
    process_terminate();

    task_next();

    panic!("No more tasks to run\n");
}
