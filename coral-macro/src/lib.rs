use proc_macro::TokenStream;

mod env_assign;
mod trace_log;

#[proc_macro]
pub fn env_assign_basic(input: TokenStream) -> TokenStream {
    env_assign::assign_basic(input)
}

#[proc_macro_derive(EnvAssign)]
pub fn derive(input: TokenStream) -> TokenStream {
    env_assign::assign_struct(input)
}

#[proc_macro]
pub fn trace_error(input: TokenStream) -> TokenStream {
    trace_log::parse_log(input, trace_log::Level::Error)
}
#[proc_macro]
pub fn trace_warn(input: TokenStream) -> TokenStream {
    trace_log::parse_log(input, trace_log::Level::Warn)
}
#[proc_macro]
pub fn trace_info(input: TokenStream) -> TokenStream {
    trace_log::parse_log(input, trace_log::Level::Info)
}
#[proc_macro]
pub fn trace_debug(input: TokenStream) -> TokenStream {
    trace_log::parse_log(input, trace_log::Level::Debug)
}
#[proc_macro]
pub fn trace_trace(input: TokenStream) -> TokenStream {
    trace_log::parse_log(input, trace_log::Level::Trace)
}
