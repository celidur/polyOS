use core::{ffi::c_void, ptr::null_mut};

use crate::bindings::{self, reboot, shutdown};

#[unsafe(no_mangle)]
pub extern "C" fn int80h_command19_reboot(_frame: *mut bindings::interrupt_frame) -> *mut c_void {
    unsafe { reboot() };
    null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn int80h_command20_shutdown(_frame: *mut bindings::interrupt_frame) -> *mut c_void {
    unsafe { shutdown() };
    null_mut()
}
