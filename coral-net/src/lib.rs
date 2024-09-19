pub mod client;
pub mod discover;
pub mod error;
pub mod hand;
pub mod midware;
pub mod server;
pub mod tcp;
pub mod tls;
pub mod udp;

pub static HTTP_HEADER_TRACE_ID: &'static str = "x-trace-id";
pub static HTTP_HEADER_SPAN_ID: &'static str = "x-span-id";
pub static HTTP_HEADER_WEBSOCKET_CONNECTION: &'static str = "upgrade";
pub static HTTP_HEADER_WEBSOCKET_UPGRADE: &'static str = "websocket";
