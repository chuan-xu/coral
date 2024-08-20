use proc_macro::TokenStream;

// pub mod trace_log;

#[proc_macro]
pub fn info(input: TokenStream) -> TokenStream {
    eprintln!("{:?}", input);
    input
}
