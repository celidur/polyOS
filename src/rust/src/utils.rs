use crate::kernel::KERNEL;

#[unsafe(no_mangle)]
pub extern "C" fn sync() {
    KERNEL.sync();
}
