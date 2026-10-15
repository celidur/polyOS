mod allocator;
mod page;
mod page_directory;

pub use allocator::{init_heap, print_memory, serial_print_memory};
pub use page::Page;
pub use page_directory::{PageDirectory, enable_paging, flags::*};

pub(crate) mod user;

pub use allocator::allocated_bytes;
