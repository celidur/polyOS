#![no_std]
#![no_main]

use polyos_std::*;

#[polyos_std::main]
fn main() {
    println!("Hello, World!");
    let mut vec = vec![1, 2, 3];

    vec.push(4);

    println!("{:?}", vec);

    let msg = format!("Hello, {}, slice: {:?}", "World", &vec[1..3]);
    serial_println!("{}", msg);
}
