use crate::{
    constant::{PIT_BASE_FREQUENCY_HZ, TIMER_HZ},
    device::{DeviceDriver, DeviceProbeStage, io::outb},
};

const PIT_COMMAND_PORT: u16 = 0x43;
const PIT_CHANNEL0_PORT: u16 = 0x40;

const PIT_ACCESS_LO_HI: u8 = 0x30;
const PIT_MODE_RATE_GENERATOR: u8 = 0x04;
const PIT_BINARY_MODE: u8 = 0x00;

pub struct TimerDriver;

impl TimerDriver {
    pub const fn new() -> Self {
        Self
    }

    fn configure(&self) {
        let divisor_u32 = (PIT_BASE_FREQUENCY_HZ / TIMER_HZ)
            .max(1)
            .min(u16::MAX as u32);
        let divisor = divisor_u32 as u16;

        unsafe {
            // Channel 0, low/high byte, mode 2 (rate generator), binary counter.
            outb(
                PIT_COMMAND_PORT,
                PIT_ACCESS_LO_HI | PIT_MODE_RATE_GENERATOR | PIT_BINARY_MODE,
            );
            outb(PIT_CHANNEL0_PORT, (divisor & 0x00FF) as u8);
            outb(PIT_CHANNEL0_PORT, (divisor >> 8) as u8);
        }

        serial_println!(
            "timer: PIT configured at {} Hz (divisor={})",
            TIMER_HZ,
            divisor
        );
    }
}

pub static TIMER_DRIVER: TimerDriver = TimerDriver::new();

impl DeviceDriver for TimerDriver {
    fn name(&self) -> &'static str {
        "timer"
    }

    fn stage(&self) -> DeviceProbeStage {
        DeviceProbeStage::Normal
    }

    fn probe(&self) {
        self.configure();
    }

    fn remove(&self) {}
}

crate::register_device_driver!(TIMER_DRIVER_REG, TIMER_DRIVER);
