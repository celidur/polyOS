use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{mem, ptr};

use crate::fs::FileHandle;

#[repr(C)]
pub struct DeviceNodeRegistration {
    pub names: &'static [&'static str],
    pub open: fn() -> FileHandle,
}

impl DeviceNodeRegistration {
    pub const fn new(names: &'static [&'static str], open: fn() -> FileHandle) -> Self {
        Self { names, open }
    }
}

unsafe extern "C" {
    static __device_nodes_start: u8;
    static __device_nodes_end: u8;
}

fn nodes() -> &'static [DeviceNodeRegistration] {
    let start = ptr::addr_of!(__device_nodes_start) as usize;
    let end = ptr::addr_of!(__device_nodes_end) as usize;
    let size = mem::size_of::<DeviceNodeRegistration>();

    if end <= start || size == 0 {
        return &[];
    }

    let len = (end - start) / size;
    unsafe { core::slice::from_raw_parts(start as *const DeviceNodeRegistration, len) }
}

pub fn find(name: &str) -> Option<&'static DeviceNodeRegistration> {
    nodes()
        .iter()
        .find(|node| node.names.iter().any(|alias| *alias == name))
}

pub fn names() -> Vec<String> {
    let mut names = Vec::new();

    for node in nodes() {
        for &name in node.names {
            if names.iter().any(|existing| existing == name) {
                continue;
            }

            names.push(name.to_string());
        }
    }

    names
}

#[macro_export]
macro_rules! register_device_node {
    ($symbol:ident, [$($name:expr),+ $(,)?], $open:path) => {
        #[used]
        #[unsafe(link_section = ".device_nodes")]
        pub static $symbol: $crate::device::node::DeviceNodeRegistration =
            $crate::device::node::DeviceNodeRegistration::new(&[$($name),+], $open);
    };
}
