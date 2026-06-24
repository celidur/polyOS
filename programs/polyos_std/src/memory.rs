use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;

pub struct PolyOSAllocator;

#[repr(C)]
struct AllocationHeader {
    base: *mut u8,
}

unsafe impl GlobalAlloc for PolyOSAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 {
            return core::ptr::null_mut();
        }

        let align = layout.align().max(core::mem::align_of::<AllocationHeader>());
        let header_size = core::mem::size_of::<AllocationHeader>();
        let Some(total) = layout
            .size()
            .checked_add(align)
            .and_then(|size| size.checked_add(header_size))
        else {
            return core::ptr::null_mut();
        };
        let Ok(total) = total.try_into() else {
            return core::ptr::null_mut();
        };

        let raw = unsafe { crate::bindings::malloc(total) as *mut u8 };
        if raw.is_null() {
            return core::ptr::null_mut();
        }

        let aligned = align_up(raw as usize + header_size, align) as *mut u8;
        let header = unsafe { aligned.cast::<AllocationHeader>().sub(1) };
        unsafe {
            header.write(AllocationHeader { base: raw });
        }

        aligned
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() {
            return;
        }

        let header = unsafe { ptr.cast::<AllocationHeader>().sub(1) };
        let base = unsafe { (*header).base };
        unsafe {
            crate::bindings::free(base as *mut c_void);
        }
    }
}

#[global_allocator]
static ALLOCATOR: PolyOSAllocator = PolyOSAllocator;

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}
