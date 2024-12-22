pub fn exit(_code: i32) -> ! {
    unsafe {
        crate::bindings::polyos_exit();
    };
}
