use crate::kernel::KERNEL;

#[doc(hidden)]
pub fn _serial(args: ::core::fmt::Arguments) {
    KERNEL.serial(args);
}

#[unsafe(no_mangle)]
pub extern "C" fn serial_write(buf: *const ::core::ffi::c_char) -> ::core::ffi::c_int {
    serial_print!("{}", unsafe {
        core::ffi::CStr::from_ptr(buf).to_string_lossy()
    });

    0
}
