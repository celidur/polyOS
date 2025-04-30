use crate::serial_println;
use alloc::format;
use alloc::string::String;
use core::{
    alloc::{GlobalAlloc, Layout},
    sync::atomic::{AtomicUsize, Ordering},
};
use linked_list_allocator::LockedHeap;

pub const HEAP_START: usize = 0x1_000_000;
pub const HEAP_SIZE: usize = 100 * 1024 * 1024; // 100MB

pub struct TrackingAllocator {
    inner: LockedHeap,
    allocated: AtomicUsize, // Tracks the total allocated size.
}

impl TrackingAllocator {
    pub const fn new() -> Self {
        Self {
            inner: LockedHeap::empty(),
            allocated: AtomicUsize::new(0),
        }
    }

    pub fn init(&self, heap_start: *mut u8, heap_size: usize) {
        unsafe {
            self.inner.lock().init(heap_start, heap_size);
        }
    }

    pub fn total_allocated(&self) -> usize {
        self.allocated.load(Ordering::Relaxed)
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.inner.alloc(layout) };
        if !ptr.is_null() {
            self.allocated.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.inner.dealloc(ptr, layout) };
        self.allocated.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

pub fn init_heap() {
    ALLOCATOR.init(HEAP_START as *mut u8, HEAP_SIZE);
}

#[unsafe(no_mangle)]
pub extern "C" fn serial_print_memory() {
    serial_println!("{}", memory_usage());
}

#[unsafe(no_mangle)]
pub extern "C" fn print_memory() {
    serial_println!("{}", memory_usage());
}

fn memory_usage() -> String {
    let allocated = ALLOCATOR.total_allocated();
    let total = HEAP_SIZE;
    let left = total - allocated;

    let allocated = if allocated > 1024 * 1024 {
        format!("{:.2} MB", allocated as f64 / (1024.0 * 1024.0))
    } else if allocated > 1024 {
        format!("{:.2} KB", allocated as f64 / 1024.0)
    } else {
        format!("{allocated} bytes")
    };

    let total = if total > 1024 * 1024 {
        format!("{:.2} MB", total as f64 / (1024.0 * 1024.0))
    } else if total > 1024 {
        format!("{:.2} KB", total as f64 / 1024.0)
    } else {
        format!("{total} bytes")
    };

    let left = if left > 1024 * 1024 {
        format!("{:.2} MB", left as f64 / (1024.0 * 1024.0))
    } else if left > 1024 {
        format!("{:.2} KB", left as f64 / 1024.0)
    } else {
        format!("{left} bytes")
    };

    format!("Heap usage: {allocated} / {total} ({left} left)")
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator::new();
