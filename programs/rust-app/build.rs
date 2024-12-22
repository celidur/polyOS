fn main() {
    println!("cargo:rerun-if-changed=../stdlib/stdlib.elf");
    println!("cargo:rustc-link-arg=../stdlib/stdlib.elf");
}
