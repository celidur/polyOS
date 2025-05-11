#![no_main]
#![no_std]

use bindings::{clear_screen, malloc, print_memory, reboot, shutdown};
use polyos_std::*;
use process::run;

#[polyos_std::main]
fn main() {
    let mut buffer = [0u8; 1024];
    println!("PolyOS v2.0.0");
    loop {
        print!("> ");
        buffer.fill(0);
        let len = polyos_std::stdio::terminal_readline(&mut buffer, true);
        let buffer = core::str::from_utf8(&buffer[..len]).unwrap();
        println!();
        if buffer.is_empty() {
            continue;
        }
        match buffer {
            "memory" => unsafe {
                print_memory();
            },
            "exit" => break,
            "malloc" => {
                let ptr = unsafe { malloc(4096 * 4096) };
                println!("malloc: {:x}", ptr as u32);
            }
            "clear" => unsafe {
                clear_screen();
            },
            "reboot" => unsafe {
                reboot();
            },
            "shutdown" => unsafe {
                shutdown();
            },
            _ => {
                if run(buffer) < 0 {
                    println!("Command not found");
                }
            }
        }
    }
}
