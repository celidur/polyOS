use crate::{constant::{HEAP_ADDRESS, HEAP_SIZE_BYTES}, serial_println};
use alloc::format;
use alloc::string::String;
use core::{
    alloc::{GlobalAlloc, Layout},
    sync::atomic::{AtomicUsize, Ordering},
};
use linked_list_allocator::LockedHeap;

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
    ALLOCATOR.init(HEAP_ADDRESS as *mut u8, HEAP_SIZE_BYTES);
}

pub fn serial_print_memory() {
    serial_println!("{}", memory_usage());
}

pub fn print_memory() {
    serial_println!("{}", memory_usage());
}

fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if size < KB {
        format!("{size}B")
    } else if size < MB {
        format!("{:.2}KB", size as f64 / KB as f64)
    } else if size < GB {
        format!("{:.2}MB", size as f64 / MB as f64)
    } else {
        format!("{:.2}GB", size as f64 / GB as f64)
    }
}

fn memory_usage() -> String {
    let allocated = ALLOCATOR.total_allocated();
    let total = HEAP_SIZE_BYTES;
    let left = total - allocated;

    format!(
        "Heap usage: {} / {} ({} left)",
        format_file_size(allocated as u64),
        format_file_size(total as u64),
        format_file_size(left as u64)
    )
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator::new();
