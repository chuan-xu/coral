use axum::{extract::Request, Router};
use hyper::body::Incoming;
use tower::Service;

use crate::util::modify_path_uri;
use crate::util::WS_RESET_URI;

pub fn websocket_reset(
    mut req: Request<Incoming>,
    mut router: Router,
) -> axum::routing::future::RouteFuture<std::convert::Infallible> {
    let ori_uri = req.uri();
    let mod_uri = modify_path_uri(ori_uri, WS_RESET_URI).unwrap();
    *(req.uri_mut()) = mod_uri;
    router.call(req)
}
