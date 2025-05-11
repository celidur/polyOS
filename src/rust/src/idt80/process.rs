use core::{ffi::c_void, ptr::null_mut};

use alloc::string::ToString;

use crate::{
    bindings::{self, copy_string_from_task, MAX_PATH},
    kernel::KERNEL,
    task::{process::ProcessArguments, task::task_next},
};

#[unsafe(no_mangle)]
pub extern "C" fn int80h_command6_process_load_start(
    _frame: *mut bindings::interrupt_frame,
) -> *mut c_void {
    let res = KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return None;
        };

        let file_user_ptr = current_task.read().get_stack_item(0);
        if file_user_ptr == 0 {
            return None;
        }

        let mut filename: [u8; MAX_PATH as usize] = [0; MAX_PATH as usize];
        let res = unsafe {
            copy_string_from_task(
                current_task.read().process.page_directory as *mut u32,
                file_user_ptr as *mut c_void,
                filename.as_mut_ptr() as *mut c_void,
                filename.len() as i32,
            )
        };
        if res != 0 {
            return None;
        }

        let filename = unsafe { core::ffi::CStr::from_ptr(filename.as_ptr() as *const i8) };
        let filename = filename.to_str().unwrap_or("");
        Some((filename, current_task.read().process.pid))
    });

    if res.is_none() {
        let res = -1;
        return res as *mut c_void;
    }
    let (program_name, pid) = res.unwrap();

    let _ = KERNEL.with_process_manager(|pm| pm.spawn(program_name, Some(pid), None));

    task_next();

    null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn int80h_command7_invoke_system_command(
    _frame: *mut bindings::interrupt_frame,
) -> *mut c_void {
    let res = KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            return None;
        };

        let ptr = current_task
            .read()
            .virtual_address_to_physical(current_task.read().get_stack_item(0) as *mut c_void)
            as *mut bindings::command_argument;
        if ptr.is_null() {
            return None;
        }

        let mut args = vec![];

        let mut command = unsafe { &*ptr  };
        let str = unsafe { core::ffi::CStr::from_ptr(command.argument.as_ptr()).to_str().unwrap_or("") };
        args.push(str.to_string());
        while command.next != null_mut() {
            command = unsafe { &*(command.next as *const bindings::command_argument) };
            let str = unsafe { core::ffi::CStr::from_ptr(command.argument.as_ptr()).to_str().unwrap_or("") };
            args.push(str.to_string());
        }

        Some((
            args,
            current_task.read().process.pid,
        ))
    });

    if res.is_none() {
        let res = -1;
        return res as *mut c_void;
    }
    let (args, pid) = res.unwrap();
    let program_name = args[0].clone();

    let args = ProcessArguments {
        args
    };

    let res = KERNEL.with_process_manager(|pm| pm.spawn(program_name.as_str(), Some(pid), Some(args)));

    if res.is_err() {
        let res = -1;
        return res as *mut c_void;
    }

    task_next();

    null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn int80h_command8_get_program_arguments(
    _frame: *mut bindings::interrupt_frame,
) -> *mut c_void {
    KERNEL.with_task_manager(|tm| {
        let current_task = if let Some(t) = tm.get_current() {
            t
        } else {
            let res = -1;
            return res as *mut c_void;
        };

        let args = current_task
            .read()
            .virtual_address_to_physical(current_task.read().get_stack_item(0) as *mut c_void)
            as *mut bindings::process_argument;
        if args.is_null() {
            let res = -1;
            return res as *mut c_void;
        }

        let root_command = unsafe { &mut *args };
        root_command.argc = current_task.read().process.args.argc;
        root_command.argv = current_task.read().process.args.argv;

        null_mut()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn int80h_command9_exit(_frame: *mut bindings::interrupt_frame) -> *mut c_void {
    let res = KERNEL.with_task_manager(|tm| {
        if let Some(t) = tm.get_current() {
            Some(t.read().process.pid)
        } else {
            return None;
        }
    });

    if res.is_none() {
        let res = -1;
        return res as *mut c_void;
    }
    let pid = res.unwrap();
    let _ = KERNEL.with_process_manager(|pm| pm.remove(pid));

    task_next();

    panic!("No more tasks to run\n");
}
