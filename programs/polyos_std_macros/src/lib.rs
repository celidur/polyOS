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
        #[export_name = "main"]
        extern "C" fn rust_main() -> ! {
            // Call the main function
            #func_name();

            // Exit the program
            polyos_std::process::exit(0);
        }

        // Original main function
        #input
    };

    TokenStream::from(expanded)
}
