use core::arch::naked_asm;
use paste::paste;
use seq_macro::seq;

use crate::{
    constant::KERNEL_CODE_SELECTOR,
    interrupts::{
        callback::{
            idt_clock, idt_general_protection_fault, idt_handle_exception,
            idt_handle_exception_error, idt_page_fault,
        },
        handler::{default_handler, syscall_wrapper},
        interrupt::{InterruptHandlerKind, InterruptSource},
        utils::idt_load,
    },
};

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

        pub const IDT_TOTAL_INTERRUPTS: usize = $table_len;
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

    idt_set(0x80, syscall_wrapper);

    for i in 0..0x20 {
        let i = InterruptSource::new(i);
        match i {
            InterruptSource::Error(_) => {
                i.register(InterruptHandlerKind::Error(idt_handle_exception_error))
            }
            InterruptSource::Plain(_) => {
                i.register(InterruptHandlerKind::Plain(idt_handle_exception))
            }
        }
    }

    InterruptSource::new(0x20).register(InterruptHandlerKind::Plain(idt_clock));

    InterruptSource::new(0xE).register(InterruptHandlerKind::Error(idt_page_fault));
    InterruptSource::new(0xD).register(InterruptHandlerKind::Error(idt_general_protection_fault));

    idt_load(&idtr_descriptor);
}
