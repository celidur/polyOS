use alloc::vec::Vec;

static mut ARGC: usize = 0;
static mut ARGV: *const *const u8 = core::ptr::null();

pub fn exit(_code: i32) -> ! {
    unsafe {
        crate::bindings::_exit(_code);
    };
}

pub fn run(command: &str) -> i32 {
    let command = nul_terminated(command);
    unsafe { crate::bindings::polyos_system_run(command.as_ptr() as *const i8) }
}

pub fn execve(path: &str, args: &[&str]) -> i32 {
    let path = nul_terminated(path);
    let mut arg_storage = Vec::new();

    for arg in args {
        arg_storage.push(nul_terminated(arg));
    }

    let mut argv: Vec<*mut i8> = arg_storage
        .iter()
        .map(|arg| arg.as_ptr() as *mut i8)
        .collect();
    argv.push(core::ptr::null_mut());

    unsafe { crate::bindings::execve(path.as_ptr() as *const i8, argv.as_ptr(), core::ptr::null()) }
}

pub fn fork() -> i32 {
    unsafe { crate::bindings::fork() }
}

pub fn waitpid(pid: i32, status: &mut i32, options: i32) -> i32 {
    unsafe { crate::bindings::waitpid(pid, status as *mut i32, options) }
}

pub fn getpid() -> i32 {
    unsafe { crate::bindings::getpid() }
}

pub fn getppid() -> i32 {
    unsafe { crate::bindings::getppid() }
}

pub fn initialize(argc: i32, argv: *const *const u8) {
    unsafe {
        ARGC = argc.max(0) as usize;
        ARGV = argv;
    }
}

pub fn argc() -> usize {
    unsafe { ARGC }
}

pub fn arg(index: usize) -> Option<&'static str> {
    unsafe {
        if index >= ARGC || ARGV.is_null() {
            return None;
        }

        let arg_ptr = *ARGV.add(index) as *const i8;
        if arg_ptr.is_null() {
            return None;
        }

        let arg_cstr = core::ffi::CStr::from_ptr(arg_ptr);
        Some(arg_cstr.to_str().unwrap_or(""))
    }
}

fn nul_terminated(value: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(value.len() + 1);
    bytes.extend_from_slice(value.as_bytes());
    bytes.push(0);
    bytes
}
