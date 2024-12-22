use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;

pub struct PolyOSAllocator;

unsafe impl GlobalAlloc for PolyOSAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        crate::bindings::polyos_malloc(layout.size()) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        crate::bindings::polyos_free(ptr as *mut c_void);
    }
}

#[global_allocator]
static ALLOCATOR: PolyOSAllocator = PolyOSAllocator;