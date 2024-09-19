use std::sync::OnceLock;

static FRONT_ROOT: OnceLock<&str> = OnceLock::new();

pub async fn front_static() -> &'static str {
    "hello world"
}
