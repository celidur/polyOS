use alloc::boxed::Box;
use core::{
    ptr::null_mut,
    sync::atomic::{AtomicPtr, Ordering},
};

use spin::Mutex;

use crate::interrupts::without_interrupts;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedDeviceError {
    AlreadyProbed,
}

pub struct ManagedDevice<T> {
    ptr: AtomicPtr<T>,
    critical: Mutex<()>,
}

impl<T> ManagedDevice<T> {
    pub const fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(null_mut()),
            critical: Mutex::new(()),
        }
    }

    pub fn probe(&self, device: T) -> Result<(), ManagedDeviceError> {
        without_interrupts(|| {
            let _guard = self.critical.lock();
            if !self.ptr.load(Ordering::Acquire).is_null() {
                return Err(ManagedDeviceError::AlreadyProbed);
            }

            let ptr = Box::into_raw(Box::new(device));
            self.ptr.store(ptr, Ordering::Release);
            Ok(())
        })
    }

    pub fn remove(&self) -> Option<T> {
        without_interrupts(|| {
            let _guard = self.critical.lock();
            let ptr = self.ptr.swap(null_mut(), Ordering::AcqRel);
            if ptr.is_null() {
                None
            } else {
                Some(unsafe { *Box::from_raw(ptr) })
            }
        })
    }

    #[allow(dead_code)]
    pub fn is_present(&self) -> bool {
        !self.ptr.load(Ordering::Acquire).is_null()
    }

    #[allow(dead_code)]
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        without_interrupts(|| {
            let _guard = self.critical.lock();
            let ptr = self.ptr.load(Ordering::Acquire);
            if ptr.is_null() {
                None
            } else {
                Some(unsafe { f(&*ptr) })
            }
        })
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        without_interrupts(|| {
            let _guard = self.critical.lock();
            let ptr = self.ptr.load(Ordering::Acquire);
            if ptr.is_null() {
                None
            } else {
                Some(unsafe { f(&mut *ptr) })
            }
        })
    }
}
