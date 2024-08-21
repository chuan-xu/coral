//! Define constant

pub static HTTP_HEADER_TRACE_ID: &'static str = "x-trace-id";

pub static HTTP_HEADER_SPAN_ID: &'static str = "x-span-id";

pub static HTTP_HEADER_WEBSOCKET_CONNECTION: &'static str = "upgrade";

pub static HTTP_HEADER_WEBSOCKET_UPGRADE: &'static str = "websocket";

// cache(redis)
pub static REDIS_KEY_NOTIFY: &'static str = "svc_update";

pub static REDIS_KEY_DISCOVER: &'static str = "svc_endpoints";
