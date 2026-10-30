//! x86 floating-point context. Entry stubs save hardware state before Rust runs.
use core::arch::asm;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct FpuState([u8; 512]);

impl Default for FpuState {
    fn default() -> Self {
        let mut state = Self([0; 512]);
        state.0[0..2].copy_from_slice(&0x037fu16.to_le_bytes());
        state.0[24..28].copy_from_slice(&0x1f80u32.to_le_bytes());
        state
    }
}

pub fn init() {
    let capabilities = core::arch::x86::__cpuid(1);
    assert!(
        capabilities.edx & (3 << 24) == 3 << 24,
        "CPU must support FXSAVE/FXRSTOR and SSE"
    );
    unsafe {
        let mut cr0: u32;
        asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack));
        cr0 = (cr0 & !((1 << 2) | (1 << 3))) | (1 << 1) | (1 << 5);
        asm!("mov cr0, {}", in(reg) cr0, options(nomem, nostack));
        let mut cr4: u32;
        asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack));
        cr4 |= (1 << 9) | (1 << 10);
        asm!("mov cr4, {}", in(reg) cr4, options(nomem, nostack));
        asm!("fninit", "ldmxcsr [{}]", in(reg) &0x1f80u32, options(nostack));
    }
}

// EBP retains the general-register frame; EBX retains the aligned FXSAVE area.
// Both are callee-saved by Rust's cdecl ABI. Nested kernel interrupts preserve
// their own scratch state and never overwrite the task's userspace state.
#[macro_export]
macro_rules! fpu_entry {
    () => {
        concat!(
            "mov ebp, esp\n",
            "sub esp, 528\n",
            "and esp, -16\n",
            "fxsave [esp]\n",
            "mov ebx, esp\n",
            "fninit\n",
            "push 0x1f80\n",
            "ldmxcsr [esp]\n",
            "add esp, 4\n",
        )
    };
}

#[macro_export]
macro_rules! fpu_leave {
    () => {
        "fxrstor [ebx]\nmov esp, ebp\n"
    };
}
