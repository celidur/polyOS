use std::env;
use std::path::PathBuf;
use walkdir::WalkDir;

fn main() {
    let include_dir = "../stdlib/include";

    // Gather all `.h` files in the `include` directory recursively
    let headers: Vec<_> = WalkDir::new(include_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "h")
                .unwrap_or(false)
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    if headers.is_empty() {
        panic!("No header files found in the include directory");
    }

    let bindings = bindgen::Builder::default()
        .clang_arg(format!("-I{}", include_dir))
        .clang_arg("-nostdlib")
        .use_core()
        .headers(
            headers
                .iter()
                .map(|h| h.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
        )
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("src");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
