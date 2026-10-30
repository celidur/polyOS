//! Single-CPU syscall counters and kernel execution-cycle measurements.
use alloc::{format, string::String};
use spin::Mutex;
const SLOTS: usize = 1024;
#[derive(Clone, Copy)]
struct Counter {
    calls: u64,
    cycles: u64,
    maximum: u64,
}
struct Metrics {
    counters: [Counter; SLOTS],
    active: Option<(usize, u64)>,
    switches: u64,
}
static DATA: Mutex<Metrics> = Mutex::new(Metrics {
    counters: [Counter {
        calls: 0,
        cycles: 0,
        maximum: 0,
    }; SLOTS],
    active: None,
    switches: 0,
});
fn timestamp() -> u64 {
    unsafe { core::arch::x86::_rdtsc() }
}
pub fn enter(number: u32) {
    let number = number as usize;
    if number >= SLOTS {
        return;
    }
    let mut data = DATA.lock();
    data.counters[number].calls = data.counters[number].calls.saturating_add(1);
    data.active = Some((number, timestamp()));
}
pub fn leave() {
    let mut data = DATA.lock();
    if let Some((number, start)) = data.active.take() {
        let elapsed = timestamp().saturating_sub(start);
        let counter = &mut data.counters[number];
        counter.cycles = counter.cycles.saturating_add(elapsed);
        counter.maximum = counter.maximum.max(elapsed);
    }
}
pub fn scheduled() {
    let mut data = DATA.lock();
    data.switches = data.switches.saturating_add(1);
}
pub fn render(timing: bool) -> String {
    let (counters, switches) = {
        let data = DATA.lock();
        (data.counters, data.switches)
    };
    let mut text = if timing {
        String::from("# kernel execution cycles; sleep/user time excluded; TSC is not calibrated\nsyscall calls total_cycles max_cycles\n")
    } else {
        format!("schedule_attempts {}\nsyscall calls\n", switches)
    };
    for (number, counter) in counters.iter().enumerate().filter(|(_, c)| c.calls != 0) {
        if timing {
            text.push_str(&format!(
                "{} {} {} {}\n",
                number, counter.calls, counter.cycles, counter.maximum
            ));
        } else {
            text.push_str(&format!("{} {}\n", number, counter.calls));
        }
    }
    text
}
