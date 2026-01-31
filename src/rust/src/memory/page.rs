use core::{alloc::Layout, ptr::NonNull, sync::atomic::AtomicU32};

use alloc::{
    alloc::{alloc_zeroed, dealloc},
    sync::Arc,
};

use crate::constant::PAGING_PAGE_SIZE;

#[derive(Debug)]
pub struct Page<T> {
    ptr: NonNull<T>,
    len: usize,
    size: usize, // always page-aligned size we actually allocated
    ref_count: Arc<AtomicU32>,
}

impl<T> Page<T> {
    /// Allocate sizeof<T> * `len` bytes, rounded up to PAGING_PAGE_SIZE, zero-initialized and PAGING_PAGE_SIZE-aligned.
    pub fn new(len: usize) -> Option<Self> {
        if len == 0 {
            return None;
        }

        let size = align_up(core::mem::size_of::<T>() * len, PAGING_PAGE_SIZE);
        let layout = Layout::from_size_align(size, PAGING_PAGE_SIZE).ok()?;

        // Safety: layout has non-zero size & valid alignment. alloc_zeroed returns null on OOM.
        let raw = unsafe { alloc_zeroed(layout) } as *mut T;
        let ptr = NonNull::new(raw)?;

        let ref_count = Arc::new(AtomicU32::new(1));

        Some(Page {
            ptr,
            len,
            size,
            ref_count,
        })
    }

    /// Total number of bytes in this page allocation (page-aligned).
    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    /// Borrow as an immutable slice.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        // Safety: we own `ptr` and `size` bytes were allocated.
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    /// Borrow as a mutable slice.
    #[inline]
    pub fn as_mut_slice(&self) -> &mut [T] {
        // Safety: we own `ptr` and `size` bytes were allocated.
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    /// Raw pointer for FFI. Caller must not free it; it's still owned by `self`.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Raw mutable pointer for FFI. Caller must not free it; it's still owned by `self`.
    #[inline]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    pub fn copy(&self) -> Option<Self>
    where
        T: Copy,
    {
        let layout = Layout::from_size_align(self.size, PAGING_PAGE_SIZE).ok()?;
        let raw = unsafe { alloc_zeroed(layout) } as *mut T;
        let ptr = NonNull::new(raw)?;

        let new_slice = unsafe { core::slice::from_raw_parts_mut(ptr.as_ptr(), self.len) };
        let old_slice = self.as_slice();
        new_slice.copy_from_slice(old_slice);

        let ref_count = Arc::new(AtomicU32::new(1));

        Some(Page {
            ptr,
            len: self.len,
            size: self.size,
            ref_count,
        })
    }
}

impl<T> Drop for Page<T> {
    fn drop(&mut self) {
        let ref_count = self
            .ref_count
            .fetch_sub(1, core::sync::atomic::Ordering::SeqCst);
        if ref_count > 1 {
            return;
        }

        // Safety: we’re dropping exactly the allocation we created.
        unsafe {
            // Using unchecked avoids re-panicking on invariant we already enforced.
            let layout = Layout::from_size_align_unchecked(self.size, PAGING_PAGE_SIZE);
            dealloc(self.ptr.as_ptr() as *mut u8, layout);
        }
    }
}

impl<T> Clone for Page<T> {
    fn clone(&self) -> Self {
        self.ref_count
            .fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        Page {
            ptr: self.ptr,
            size: self.size,
            len: self.len,
            ref_count: Arc::clone(&self.ref_count),
        }
    }
}

#[inline]
const fn align_up(x: usize, align: usize) -> usize {
    (x + align - 1) & !(align - 1)
}
