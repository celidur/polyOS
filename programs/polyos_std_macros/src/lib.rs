use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input as a function
    let input = parse_macro_input!(item as ItemFn);
    let func_name = &input.sig.ident;

    // Generate the entry point `main`
    let expanded = quote! {
        fn rust_main_wrapper() -> ! {
            #func_name();
            polyos_std::process::exit(0);
        }

        #[unsafe(export_name = "main")]
        extern "C" fn rust_main(argc: i32, argv: *const *const u8) -> ! {
            polyos_std::process::initialize(argc, argv);
            rust_main_wrapper()
        }

        #input
    };

    TokenStream::from(expanded)
}
