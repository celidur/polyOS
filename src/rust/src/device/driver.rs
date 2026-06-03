use core::{mem, ptr};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceProbeStage {
    Early = 0,
    Normal = 1,
}

pub trait DeviceDriver: Sync {
    #[allow(dead_code)]
    fn name(&self) -> &'static str;
    fn stage(&self) -> DeviceProbeStage;
    fn probe(&self);
    fn remove(&self);
}

#[repr(C)]
pub struct DeviceRegistration {
    pub driver: &'static dyn DeviceDriver,
}

impl DeviceRegistration {
    pub const fn new(driver: &'static dyn DeviceDriver) -> Self {
        Self { driver }
    }
}

unsafe extern "C" {
    static __device_drivers_start: u8;
    static __device_drivers_end: u8;
}

fn drivers() -> &'static [DeviceRegistration] {
    let start = ptr::addr_of!(__device_drivers_start) as usize;
    let end = ptr::addr_of!(__device_drivers_end) as usize;
    let size = mem::size_of::<DeviceRegistration>();

    if end <= start || size == 0 {
        return &[];
    }

    let len = (end - start) / size;
    unsafe { core::slice::from_raw_parts(start as *const DeviceRegistration, len) }
}

pub fn probe_stage(stage: DeviceProbeStage) {
    for driver in drivers() {
        if driver.driver.stage() == stage {
            driver.driver.probe();
        }
    }
}

#[allow(dead_code)]
pub fn probe_all() {
    probe_stage(DeviceProbeStage::Early);
    probe_stage(DeviceProbeStage::Normal);
}

#[allow(dead_code)]
pub fn remove_stage(stage: DeviceProbeStage) {
    for driver in drivers().iter().rev() {
        if driver.driver.stage() == stage {
            driver.driver.remove();
        }
    }
}

#[allow(dead_code)]
pub fn remove_all() {
    remove_stage(DeviceProbeStage::Normal);
    remove_stage(DeviceProbeStage::Early);
}

#[macro_export]
macro_rules! register_device_driver {
    ($symbol:ident, $driver:ident) => {
        #[used]
        #[unsafe(link_section = ".device_drivers")]
        pub static $symbol: $crate::device::driver::DeviceRegistration =
            $crate::device::driver::DeviceRegistration::new(&$driver);
    };
}
