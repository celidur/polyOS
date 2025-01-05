pub fn exit(_code: i32) -> ! {
    unsafe {
        crate::bindings::polyos_exit();
    };
}

pub fn run(command: &str) -> i32 {
    unsafe { crate::bindings::polyos_system_run(command.as_ptr() as *const i8) }
}
