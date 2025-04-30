extern crate alloc;

use alloc::format;
use core::panic::PanicInfo;

use crate::bindings::kernel_panic;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let msg = format!("rust panic: {info}\0");

    unsafe { kernel_panic(msg.as_ptr() as *const i8) }
}
