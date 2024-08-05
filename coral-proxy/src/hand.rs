use axum::{body::BodyDataStream, extract::Request, http::uri::PathAndQuery};
use coral_log::error;
use hyper::client::conn::http2::SendRequest;
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::sync::Arc;

use coral_runtime::tokio;

use crate::error::CoralRes;

pub struct PxyChan {
    sender: SendRequest<BodyDataStream>,
    count: Arc<()>,
}

impl PxyChan {
    pub async fn new(dst: &String) -> CoralRes<PxyChan> {
        let stream = tokio::net::TcpStream::connect(dst).await?;
        let (mut sender, conn) = hyper::client::conn::http2::Builder::new(TokioExecutor::new())
            .handshake(TokioIo::new(stream))
            .await?;

        tokio::spawn(async move {
            if let Err(e) = conn.await {
                error!(error = e.to_string(), "proxy chan conn failed");
            }
        })
        .await
        .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(3));
        let body = axum::body::Body::empty().into_data_stream();
        let req = hyper::Request::builder()
            .method("POST")
            .uri("/heartbeat")
            .body(body)?;
        let res = sender.send_request(req).await?;
        if res.status() != hyper::StatusCode::OK {
            std::panic!("invalid server addr");
        }
        Ok(Self {
            sender,
            count: Arc::default(),
        })
    }

    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.count)
    }

    pub fn get_sender(&mut self) -> &mut SendRequest<BodyDataStream> {
        &mut self.sender
    }
}

impl Clone for PxyChan {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            count: self.count.clone(),
        }
    }
}

pub async fn proxy(req: Request) -> hyper::Response<hyper::body::Incoming> {
    let uri = req.extensions().get::<PathAndQuery>().unwrap().clone();
    let mut pxy_ch = req.extensions().get::<PxyChan>().unwrap().clone();
    let headers = req.headers().clone();
    let body = req.into_body().into_data_stream();
    let mut pxy_builder = hyper::Request::builder().method("POST").uri(uri);
    let pxy_headers = pxy_builder.headers_mut().unwrap();
    *pxy_headers = headers;
    let pxy_req = pxy_builder.body(body).unwrap();
    let rsp = pxy_ch.get_sender().send_request(pxy_req).await.unwrap();
    rsp
}
