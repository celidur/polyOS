use crate::interrupts::{
    idt::IDT_TOTAL_INTERRUPTS,
    interrupt_frame::InterruptFrame,
    irq_numbers::{InterruptErrorNumber, InterruptNumber},
};
use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::RwLock;

pub type InterruptHandler = fn(&InterruptFrame);
pub type InterruptErrorHandler = fn(&InterruptFrame, u32);

lazy_static! {
    static ref INT_CALLBACKS: Arc<RwLock<[Option<InterruptHandler>; IDT_TOTAL_INTERRUPTS]>> =
        Arc::new(RwLock::new([None; IDT_TOTAL_INTERRUPTS]));
    static ref INT_ERR_CALLBACKS: Arc<RwLock<[Option<InterruptErrorHandler>; IDT_TOTAL_INTERRUPTS]>> =
        Arc::new(RwLock::new([None; IDT_TOTAL_INTERRUPTS]));
}

pub trait RegisterInterrupt {
    type Callback;

    fn register(self, cb: Self::Callback);
    fn get_callback(&self) -> Option<Self::Callback>;
}

impl RegisterInterrupt for InterruptErrorNumber {
    type Callback = InterruptErrorHandler;

    fn register(self, cb: Self::Callback) {
        let mut int_err_callbacks = INT_ERR_CALLBACKS.write();
        int_err_callbacks[self.index()] = Some(cb);
    }

    fn get_callback(&self) -> Option<InterruptErrorHandler> {
        let int_err_callbacks = INT_ERR_CALLBACKS.read();
        int_err_callbacks[self.index()]
    }
}

impl RegisterInterrupt for InterruptNumber {
    type Callback = InterruptHandler;

    fn register(self, cb: Self::Callback) {
        let mut int_callbacks = INT_CALLBACKS.write();
        int_callbacks[self.index()] = Some(cb);
    }

    fn get_callback(&self) -> Option<InterruptHandler> {
        let int_callbacks = INT_CALLBACKS.read();
        int_callbacks[self.index()]
    }
}
