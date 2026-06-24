use crate::{
    interrupts::InterruptFrame,
    utils::{reboot, shutdown},
};

use super::abi;

const LINUX_REBOOT_MAGIC1: u32 = 0xfee1dead;
const LINUX_REBOOT_MAGIC2: u32 = 672274793;
const LINUX_REBOOT_CMD_RESTART: u32 = 0x01234567;
const LINUX_REBOOT_CMD_HALT: u32 = 0xcdef0123;
const LINUX_REBOOT_CMD_POWER_OFF: u32 = 0x4321fedc;

pub fn syscall_linux_reboot(_frame: &InterruptFrame) -> u32 {
    let Some((magic1, magic2, cmd)) = crate::kernel::KERNEL.with_task_manager(|tm| {
        let current_task = tm.get_current()?;
        let task = current_task.read();
        Some((
            task.get_stack_item(0),
            task.get_stack_item(1),
            task.get_stack_item(2),
        ))
    }) else {
        return abi::errno(abi::ESRCH);
    };

    if magic1 != LINUX_REBOOT_MAGIC1 || magic2 != LINUX_REBOOT_MAGIC2 {
        return abi::errno(abi::EINVAL);
    }

    match cmd {
        LINUX_REBOOT_CMD_RESTART => reboot(),
        LINUX_REBOOT_CMD_HALT | LINUX_REBOOT_CMD_POWER_OFF => shutdown(),
        _ => return abi::errno(abi::EINVAL),
    }

    0
}

pub fn syscall_kernel_selftest(_frame: &InterruptFrame) -> u32 {
    crate::kernel_selftest::run()
}
