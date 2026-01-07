use alloc::{boxed::Box, vec::Vec};
use core::ptr;

static mut ARGS_PTR: *const &'static str = ptr::null();
static mut ARGS_LEN: usize = 0;

pub fn exit(_code: i32) -> ! {
    unsafe {
        crate::bindings::polyos_exit();
    };
}

pub fn run(command: &str) -> i32 {
    unsafe { crate::bindings::polyos_system_run(command.as_ptr() as *const i8) }
}

pub fn initialize(argc: i32, argv: *const *const u8) {
    let argc = argc.max(0) as usize;
    let mut v: Vec<&'static str> = Vec::with_capacity(argc as usize);

    for i in 0..argc {
        let arg_ptr = unsafe { *argv.add(i as usize) };
        if arg_ptr.is_null() {
            v.push("");
            continue;
        }
        let arg_cstr = unsafe { core::ffi::CStr::from_ptr(arg_ptr as *const i8) };
        let arg_str = arg_cstr.to_str().unwrap_or("");

        let s_static: &'static str = unsafe { core::mem::transmute::<&str, &'static str>(arg_str) };

        v.push(s_static);
    }

    let boxed: Box<[&'static str]> = v.into_boxed_slice();
    let slice: &'static [&'static str] = Box::leak(boxed);

    unsafe {
        ARGS_PTR = slice.as_ptr();
        ARGS_LEN = slice.len();
    }
}

pub fn args() -> &'static [&'static str] {
    unsafe {
        if ARGS_PTR.is_null() {
            &[]
        } else {
            core::slice::from_raw_parts(ARGS_PTR, ARGS_LEN)
        }
    }
}