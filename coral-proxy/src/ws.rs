#![allow(unused)]
use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::Request;
use axum::http::HeaderValue;
use axum::response::Response;
use futures::SinkExt;
use futures::StreamExt;
use futures::TryStreamExt;
use hyper::body::Incoming;
use hyper::header::CONNECTION;
use hyper::header::SEC_WEBSOCKET_ACCEPT;
use hyper::header::SEC_WEBSOCKET_KEY;
use hyper::header::UPGRADE;
use hyper::StatusCode;
use hyper_util::rt::TokioIo;
use log::error;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::WebSocketStream;

use crate::error::CoralRes;
use crate::error::Error;

// TODO
pub async fn websocket_conn_hand(mut req: Request<Incoming>, addr: SocketAddr) {
    match hyper::upgrade::on(&mut req).await {
        Ok(io) => {
            let stream = TokioIo::new(io);
            let ws_stream = WebSocketStream::from_raw_socket(stream, Role::Server, None).await;
            let (mut outgoing, mut incoming) = ws_stream.split();
            // incoming.map(|v| match v {
            //     Ok(msg) => todo!(),
            //     Err(err) => todo!(),
            // })
            if let Some(res) = incoming.next().await {
                match res {
                    Ok(msg) => {
                        if let Err(e) = outgoing.send(msg).await {}
                        if let Err(e) = outgoing.flush().await {}
                    }
                    Err(err) => error!(e = err.to_string(); "failed to receive websocket mes"),
                }
            }
            // incoming
            //     .try_for_each(|message| outgoing.send(message))
            //     .await
            //     .unwrap();
            // incoming.forward(outgoing).await.unwrap();
        }
        Err(err) => {
            error!(e = err.to_string(); "fail to upgrade in websocket stream");
        }
    }
}

// pub fn websocket_hand(req: Request) -> CoralRes<hyper::Response<hyper::body::Incoming>> {
pub async fn websocket_upgrade_hand(req: Request) -> CoralRes<Response<Body>> {
    let key = req
        .headers()
        .get(SEC_WEBSOCKET_KEY)
        .ok_or(Error::MissingHeader("sec-websocket-key"))?;
    let derived_key = derive_accept_key(key.as_bytes());
    let derived_hv = HeaderValue::from_str(&derived_key)?;
    let mut res = Response::new(Body::default());
    *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    *res.version_mut() = req.version();
    res.headers_mut().append(
        CONNECTION,
        HeaderValue::from_static(coral_util::consts::HTTP_HEADER_WEBSOCKET_CONNECTION),
    );
    res.headers_mut().append(
        UPGRADE,
        HeaderValue::from_static(coral_util::consts::HTTP_HEADER_WEBSOCKET_UPGRADE),
    );
    res.headers_mut().append(SEC_WEBSOCKET_ACCEPT, derived_hv);
    Ok(res)
}
