use crate::{
    interrupts::idt::InterruptFrame,
    utils::{reboot, shutdown},
};

pub fn int80h_command19_reboot(_frame: &InterruptFrame) -> u32 {
    reboot();
    0
}

pub fn int80h_command20_shutdown(_frame: &InterruptFrame) -> u32 {
    shutdown();
    0
}
