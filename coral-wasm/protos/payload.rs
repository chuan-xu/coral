#[wasm_bindgen::prelude::wasm_bindgen]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Token1 {
    #[prost(string, repeated, tag = "1")]
    pub(crate) keys: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(string, repeated, tag = "2")]
    pub(crate) vals: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(string, tag = "3")]
    pub(crate) iss: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub(crate) sub: ::prost::alloc::string::String,
    #[prost(fixed64, tag = "5")]
    pub(crate) exp: u64,
}
#[wasm_bindgen::prelude::wasm_bindgen]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Request {
    #[prost(message, optional, tag = "1")]
    pub(crate) token: ::core::option::Option<Token>,
    #[prost(bytes = "vec", optional, tag = "2")]
    pub(crate) signature: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
    #[prost(bytes = "vec", tag = "3")]
    pub(crate) payload: ::prost::alloc::vec::Vec<u8>,
}
#[wasm_bindgen::prelude::wasm_bindgen]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    #[prost(bytes = "vec", tag = "1")]
    pub(crate) payload: ::prost::alloc::vec::Vec<u8>,
}
