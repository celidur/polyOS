use core::alloc::{GlobalAlloc, Layout};

use crate::bindings::{kernel_panic, kfree, kmalloc};
struct KernelAllocator;

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = kmalloc(layout.size()) as *mut u8;
        if ptr.is_null() {
            // Allocation failed
            let msg = c"Kernel allocator failed to allocate memory".as_ptr();
            kernel_panic(msg);
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        kfree(ptr as *mut core::ffi::c_void);
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: KernelAllocator = KernelAllocator;
