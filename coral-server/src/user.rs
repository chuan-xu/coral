use axum::extract::Request;

/// nickname: must
/// phonenum: must
/// username: option(ext)
/// captcha
pub async fn register(req: Request) {}

/// proof of work
/// get captcha
pub async fn captcha(req: Request) {}

pub async fn login(req: Request) {}
