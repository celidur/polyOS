use crate::interrupts::{
    irq_numbers::{InterruptErrorNumber, InterruptNumber},
    register::{InterruptErrorHandler, InterruptHandler, RegisterInterrupt},
};

pub enum InterruptSource {
    Plain(InterruptNumber),
    Error(InterruptErrorNumber),
}

pub enum InterruptHandlerKind {
    Plain(InterruptHandler),
    Error(InterruptErrorHandler),
}

impl InterruptSource {
    pub fn new(vector: u16) -> Self {
        if let Some(err) = InterruptErrorNumber::from_u8(vector as u8) {
            Self::Error(err)
        } else {
            Self::Plain(InterruptNumber::new(vector))
        }
    }

    pub fn register(self, handler: InterruptHandlerKind) {
        match (self, handler) {
            (Self::Plain(v), InterruptHandlerKind::Plain(f)) => v.register(f),
            (Self::Error(e), InterruptHandlerKind::Error(f)) => e.register(f),
            _ => panic!("mismatched interrupt handler type"),
        }
    }
}
