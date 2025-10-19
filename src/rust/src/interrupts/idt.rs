use alloc::sync::Arc;
use core::arch::{asm, naked_asm};
use lazy_static::lazy_static;
use paste::paste;
use seq_macro::seq;
use spin::RwLock;

use crate::{
    bindings::kernel_page,
    device::io::outb,
    interrupts::idt80::SyscallId,
    kernel::KERNEL_CODE_SELECTOR,
    schedule::{
        process::process_terminate,
        task::{task_current_save_state, task_next, task_page},
    },
    utils::sync,
};

pub type InterruptCb = fn(frame: &InterruptFrame);
pub type InterruptErrCb = fn(frame: &InterruptFrame, error_code: u32);

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntVecErr {
    DF = 8,  // Double Fault
    TS = 10, // Invalid TSS
    NP = 11, // Segment Not Present
    SS = 12, // Stack Segment Fault
    GP = 13, // General Protection
    PF = 14, // Page Fault
}

impl IntVecErr {
    #[inline(always)]
    pub const fn as_index(self) -> usize {
        self as u8 as usize
    }

    pub const fn new(v: u8) -> Option<Self> {
        match v {
            8 => Some(Self::DF),
            10 => Some(Self::TS),
            11 => Some(Self::NP),
            12 => Some(Self::SS),
            13 => Some(Self::GP),
            14 => Some(Self::PF),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntVec(pub u16);

impl IntVec {
    #[inline(always)]
    pub const fn new(v: u16) -> Self {
        Self(v)
    }
    #[inline(always)]
    pub const fn as_index(self) -> usize {
        self.0 as usize
    }
}

pub enum InterruptCallback {
    Plain(InterruptCb),
    Err(InterruptErrCb),
}

pub enum Interrupt {
    Plain(IntVec),
    Err(IntVecErr),
}

impl Interrupt {
    pub fn new(v: u16) -> Self {
        if let Some(err_vec) = IntVecErr::new(v as u8) {
            Interrupt::Err(err_vec)
        } else {
            Interrupt::Plain(IntVec::new(v))
        }
    }

    pub fn register(self, cb: InterruptCallback) {
        match (self, cb) {
            (Interrupt::Plain(v), InterruptCallback::Plain(f)) => v.register(f),
            (Interrupt::Err(e), InterruptCallback::Err(f)) => e.register(f),
            _ => panic!("mismatched interrupt callback type"),
        }
    }
}

pub type SyscallHandler = fn(frame: &InterruptFrame) -> u32;
const MAX_SYSCALLS: usize = 256;

lazy_static! {
    static ref INT_CALLBACKS: Arc<RwLock<[Option<InterruptCb>; IDT_TOTAL_INTERRUPTS]>> =
        Arc::new(RwLock::new([None; IDT_TOTAL_INTERRUPTS]));
    static ref INT_ERR_CALLBACKS: Arc<RwLock<[Option<InterruptErrCb>; IDT_TOTAL_INTERRUPTS]>> =
        Arc::new(RwLock::new([None; IDT_TOTAL_INTERRUPTS]));
    static ref SYSCALL_TABLE: Arc<RwLock<[Option<SyscallHandler>; MAX_SYSCALLS]>> =
        Arc::new(RwLock::new([None; MAX_SYSCALLS]));
}

#[inline]
pub fn syscall_register(id: SyscallId, handler: SyscallHandler) {
    let mut table = SYSCALL_TABLE.write();
    table[id as usize] = Some(handler);
}

#[inline]
pub fn syscall_get_handler(id: SyscallId) -> Option<SyscallHandler> {
    let table = SYSCALL_TABLE.read();
    table[id as usize]
}

pub trait RegisterInterrupt {
    type Callback;

    fn register(self, cb: Self::Callback);
    fn get_callback(&self) -> Option<Self::Callback>;
}

impl RegisterInterrupt for IntVec {
    type Callback = InterruptCb;

    fn register(self, cb: Self::Callback) {
        let mut int_callbacks = INT_CALLBACKS.write();
        int_callbacks[self.as_index()] = Some(cb);
    }

    fn get_callback(&self) -> Option<InterruptCb> {
        let int_callbacks = INT_CALLBACKS.read();
        int_callbacks[self.as_index()]
    }
}

impl RegisterInterrupt for IntVecErr {
    type Callback = InterruptErrCb;

    fn register(self, cb: Self::Callback) {
        let mut int_err_callbacks = INT_ERR_CALLBACKS.write();
        int_err_callbacks[self.as_index()] = Some(cb);
    }

    fn get_callback(&self) -> Option<InterruptErrCb> {
        let int_err_callbacks = INT_ERR_CALLBACKS.read();
        int_err_callbacks[self.as_index()]
    }
}

#[repr(C, packed)]
pub struct Idtr {
    pub limit: u16,
    pub base: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtDesc {
    pub offset_1: u16,
    pub selector: u16,
    pub zero: u8,
    pub type_attr: u8,
    pub offset_2: u16,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct InterruptFrame {
    pub edi: u32,
    pub esi: u32,
    pub ebp: u32,
    pub reserved: u32,
    pub ebx: u32,
    pub edx: u32,
    pub ecx: u32,
    pub eax: u32,

    pub ip: u32, // instruction pointer
    pub cs: u32,
    pub flags: u32,
    pub esp: u32,
    pub ss: u32,
}

#[unsafe(no_mangle)]
pub extern "C" fn idt_load(idtr: &Idtr) {
    unsafe {
        asm!(
            "lidt [{0}]",
            in(reg) idtr,
            options(nostack, readonly),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_cr2() -> u32 {
    let cr2: u32;
    unsafe {
        asm!("mov {}, cr2", out(reg) cr2);
    }
    cr2
}

#[unsafe(no_mangle)]
pub extern "C" fn enable_interrupts() {
    unsafe { asm!("sti", options(nostack)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn disable_interrupts() {
    unsafe { asm!("cli", options(nostack)) }
}

#[inline(always)]
fn eoi_pic1() {
    unsafe { outb(0x20, 0x20) };
}

#[unsafe(no_mangle)]
pub extern "C" fn interrupt_handler(interrupt: u32, frame: &InterruptFrame) {
    unsafe { kernel_page() };
    task_current_save_state(frame);
    if let Interrupt::Plain(vec) = Interrupt::new(interrupt as u16)
        && let Some(cb) = vec.get_callback()
    {
        cb(frame);
    }

    task_page();
    eoi_pic1();
}

#[unsafe(no_mangle)]
pub extern "C" fn interrupt_handler_error(error_code: u32, interrupt: u32, frame: &InterruptFrame) {
    unsafe { kernel_page() };
    task_current_save_state(frame);
    if let Interrupt::Err(vec) = Interrupt::new(interrupt as u16)
        && let Some(cb) = vec.get_callback()
    {
        cb(frame, error_code);
    }

    task_page();
    eoi_pic1();
}

#[unsafe(no_mangle)]
pub extern "C" fn int80h_handler(frame: &mut InterruptFrame) -> u32 {
    unsafe { kernel_page() };
    task_current_save_state(frame);
    let res = int80h_handle_command(frame);
    frame.eax = res;
    task_current_save_state(frame);
    task_page();
    res
}

#[unsafe(naked)]
pub extern "C" fn int80h_wrapper() {
    #[allow(unused_unsafe)]
    unsafe {
        naked_asm!(
            "
        pushad
        push esp
        call int80h_handler
        add esp, 4
        mov [esp + 28], eax
        popad
        iretd
        ",
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn default_handler() {
    panic!("Unhandled interrupt");
}

#[macro_export]
macro_rules! __gen_dispatch {
    ($num:literal, true) => {
        concat!(
            "mov eax, [esp+40]\n",
            "push eax\n",
            "call interrupt_handler_error\n",
            "add esp, 12\n",
        )
    };
    ($num:literal, false) => {
        concat!("call interrupt_handler\n", "add esp, 8\n",)
    };
}

macro_rules! def_interrupts {
    (
        // list only the vectors that push an error code
        error_handlers: { $($err:literal),* $(,)? },

        // table name and total size
        table_name: $table_ident:ident,
        size: $table_len:expr $(,)?
    ) => {
        paste! {
            macro_rules! [<__dispatch_ $table_ident>] {
                $(
                    ($err) => {
                        __gen_dispatch!($err, true)
                    };
                )*
                ($num:literal) => {
                    __gen_dispatch!($num, false)
                };
            }
        }

        /* ---- stubs ---- */
        seq!(N in 0..$table_len {
            paste! {
                #[unsafe(naked)]
                pub unsafe extern "C" fn [<int N>]() {
                    naked_asm!(
                        concat!(
                            "pushad\n",
                            "push esp\n",
                            "push ", stringify!(N), "\n",
                            [<__dispatch_ $table_ident>]!(N),
                            "popad\n",
                            "iretd\n",
                        ),
                    );
                }
            }
        });

        pub static $table_ident: [unsafe extern "C" fn(); $table_len] = {
            let mut tbl: [unsafe extern "C" fn(); $table_len] = [default_handler as unsafe extern "C" fn(); $table_len];
            seq!(N in 0..$table_len {
                paste! {
                    tbl[N] = [<int N>] as unsafe extern "C" fn();
                }
            });
            tbl
        };

        pub static mut IDT_DESCRIPTORS: [IdtDesc; $table_len] = [IdtDesc {
            offset_1: 0,
            selector: 0,
            zero: 0,
            type_attr: 0,
            offset_2: 0,
        }; $table_len];

        const IDT_TOTAL_INTERRUPTS: usize = $table_len;
    };
}

def_interrupts! {
    error_handlers: { 8, 10, 11, 12, 13, 14 },
    table_name: INTERRUPT_POINTER_TABLE,
    size: 512,
}

fn idt_set(interrupt_no: usize, address: unsafe extern "C" fn()) {
    let addr = address as u32;
    let desc: &mut IdtDesc = unsafe { &mut IDT_DESCRIPTORS[interrupt_no] };
    desc.offset_1 = (addr & 0xFFFF) as u16;
    desc.offset_2 = ((addr >> 16) & 0xFFFF) as u16;
    desc.selector = KERNEL_CODE_SELECTOR;
    desc.zero = 0x00;
    desc.type_attr = 0xEE;
}

pub fn idt_init() {
    let idtr_descriptor = Idtr {
        base: { &raw const IDT_DESCRIPTORS } as u32,
        limit: (IDT_TOTAL_INTERRUPTS * core::mem::size_of::<IdtDesc>() - 1) as u16,
    };

    for (i, handler) in INTERRUPT_POINTER_TABLE.iter().enumerate() {
        idt_set(i, *handler);
    }

    idt_set(0x80, int80h_wrapper);

    for i in 0..0x20 {
        let i = Interrupt::new(i);
        match i {
            Interrupt::Err(_) => i.register(InterruptCallback::Err(idt_handle_exception_error)),
            Interrupt::Plain(_) => i.register(InterruptCallback::Plain(idt_handle_exception)),
        }
    }

    Interrupt::new(0x20).register(InterruptCallback::Plain(idt_clock));

    Interrupt::new(0xE).register(InterruptCallback::Err(idt_page_fault));
    Interrupt::new(0xD).register(InterruptCallback::Err(idt_general_protection_fault));

    idt_load(&idtr_descriptor);
}

fn idt_handle_exception(_frame: &InterruptFrame) {
    process_terminate();
    task_next();
    panic!("No more tasks to run\n");
}

fn idt_handle_exception_error(_frame: &InterruptFrame, _error_code: u32) {
    process_terminate();
    task_next();
    panic!("No more tasks to run\n");
}

fn int80h_handle_command(frame: &InterruptFrame) -> u32 {
    let cmd = frame.eax;
    let cmd = match SyscallId::new(cmd as u8) {
        Some(c) => c,
        None => {
            serial_println!("Unknown syscall command: {}", cmd);
            return u32::MAX;
        }
    };

    syscall_get_handler(cmd)
        .map(|handler| handler(frame))
        .unwrap_or_else(|| {
            serial_println!("Unknown syscall command: {:?}", cmd);
            u32::MAX
        })
}

fn idt_clock(frame: &InterruptFrame) {
    unsafe { kernel_page() };
    task_current_save_state(frame);

    sync();
}

fn idt_page_fault(frame: &InterruptFrame, code_error: u32) {
    let faulting_address = get_cr2();

    let p = code_error & 0x1;
    let w = (code_error >> 1) & 0x1;
    let u = (code_error >> 2) & 0x1;
    let r = (code_error >> 3) & 0x1;
    let i = (code_error >> 4) & 0x1;
    let pk = (code_error >> 5) & 0x1;
    let ss = (code_error >> 6) & 0x1;
    let sgx = (code_error >> 15) & 0x1;

    serial_print!("Page fault( ");
    if p != 0 {
        serial_print!("protection violation ");
    }
    if w != 0 {
        serial_print!("write ");
    } else {
        serial_print!("read ");
    }
    if u != 0 {
        serial_print!("user ");
    } else {
        serial_print!("supervisor ");
    }
    if r != 0 {
        serial_print!("reserved ");
    }
    if i != 0 {
        serial_print!("instruction fetch ");
    }
    if pk != 0 {
        serial_print!("protection key violation ");
    }
    if ss != 0 {
        serial_print!("shadow stack ");
    }
    if sgx != 0 {
        serial_print!("SGX ");
    }
    serial_println!(") at 0x{:x}", faulting_address);

    serial_println!("Register:");
    serial_println!("{:?}", frame);

    panic!("Page fault");
}

fn idt_general_protection_fault(_frame: &InterruptFrame, code_error: u32) {
    serial_println!("General protection fault");
    let e = code_error & 0x1;
    if e != 0 {
        serial_println!("the exception originated externally to the processor");
    } else {
        let tbl = (code_error >> 1) & 0x3;
        let index = (code_error >> 3) & 0x1FFF;
        match tbl {
            0 => serial_print!("GDT"),
            1 | 3 => serial_print!("IDT"),
            2 => serial_print!("LDT"),
            _ => {}
        }
        serial_println!(" index: 0x{:x}", index);
    }
    panic!("General protection fault");
}
