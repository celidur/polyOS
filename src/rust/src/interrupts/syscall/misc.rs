use crate::{
    interrupts::InterruptFrame,
    utils::{reboot, shutdown},
};

pub fn syscall_reboot(_frame: &InterruptFrame) -> u32 {
    reboot();
    0
}

pub fn syscall_shutdown(_frame: &InterruptFrame) -> u32 {
    shutdown();
    0
}
