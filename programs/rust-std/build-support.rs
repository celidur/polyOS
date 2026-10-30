fn main() {
    if std::env::var("CARGO_CFG_TARGET_VENDOR").as_deref() != Ok("polyos") {
        return;
    }
    let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let runtime = manifest
        .ancestors()
        .map(|p| p.join("programs/rust-std"))
        .find(|p| p.is_dir())
        .expect("PolyOS runtime not found");
    for file in [
        "linker.sh",
        "linker.ld",
        "lib/runtime.o",
        "lib/start.o",
        "lib/libc.a",
    ] {
        println!("cargo:rerun-if-changed={}", runtime.join(file).display());
    }
}
