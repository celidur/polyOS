use core::{alloc::Layout, mem, ptr::NonNull};

use alloc::alloc::{alloc_zeroed, dealloc};

use crate::constant::PAGING_PAGE_SIZE;

#[derive(Debug)]
pub struct Page {
    ptr: NonNull<u8>,
    size: usize, // always page-aligned size we actually allocated
}

impl Page {
    /// Allocate `size` bytes, rounded up to PAGING_PAGE_SIZE, zero-initialized and PAGING_PAGE_SIZE-aligned.
    pub fn new(size: usize) -> Option<Self> {
        if size == 0 {
            return None;
        }
        let size = align_up(size, PAGING_PAGE_SIZE);
        let layout = Layout::from_size_align(size, PAGING_PAGE_SIZE).ok()?;

        // Safety: layout has non-zero size & valid alignment. alloc_zeroed returns null on OOM.
        let raw = unsafe { alloc_zeroed(layout) };
        let ptr = NonNull::new(raw)?;

        Some(Page { ptr, size })
    }

    /// Total number of bytes in this page allocation (page-aligned).
    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    /// Borrow as an immutable slice.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        // Safety: we own `ptr` and `size` bytes were allocated.
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.size) }
    }

    /// Borrow as a mutable slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // Safety: we own `ptr` and `size` bytes were allocated.
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size) }
    }

    /// Raw pointer for FFI. Caller must not free it; it's still owned by `self`.
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    /// Raw mutable pointer for FFI. Caller must not free it; it's still owned by `self`.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// Transfer ownership out to FFI. You **must** later call `Page::from_raw`
    /// (or equivalent) to free, or you’ll leak.
    pub fn into_raw(self) -> (*mut u8, usize) {
        let p = self.ptr.as_ptr();
        let n = self.size;
        mem::forget(self);
        (p, n)
    }

    /// Recreate a `Page` from a raw pointer and size that were produced by `into_raw`.
    ///
    /// # Safety
    /// - `ptr` must have been returned by `into_raw` for *this same* allocation strategy.
    /// - `size` must equal the page-aligned size originally allocated.
    /// - No other owner may free `ptr`.
    pub unsafe fn from_raw(ptr: *mut u8, size: usize) -> Self {
        Page {
            ptr: NonNull::new(ptr).expect("null ptr"),
            size,
        }
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        // Safety: we’re dropping exactly the allocation we created.
        unsafe {
            // Using unchecked avoids re-panicking on invariant we already enforced.
            let layout = Layout::from_size_align_unchecked(self.size, PAGING_PAGE_SIZE);
            dealloc(self.ptr.as_ptr(), layout);
        }
    }
}

#[inline]
const fn align_up(x: usize, align: usize) -> usize {
    (x + align - 1) & !(align - 1)
}
