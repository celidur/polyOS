#[macro_export]
macro_rules! entry_point {
    ($path:path) => {
        #[unsafe(export_name = "kernel_main")]
        pub extern "C" fn __impl_start() -> ! {
            // validate the signature of the program entry point
            let f: fn() -> ! = $path;

            f()
        }
    };
}
