use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::error::CoralRes;

#[async_trait::async_trait]
pub trait Request<R, H> {
    async fn send(&mut self, req: hyper::Request<R>) -> CoralRes<hyper::Response<H>>;
}

#[async_trait::async_trait]
pub trait Pool {
    // fn add(&mut self) -> CoralRes<()>;
    type Client;

    async fn load_balance(self: Arc<Self>) -> CoralRes<Option<Self::Client>>;
}

pub struct StatisticsGuard(pub(crate) Arc<AtomicUsize>);

impl Drop for StatisticsGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

pub trait statistics {
    fn usage_count(&self) -> usize;

    fn usage_add(&self) -> StatisticsGuard;
}
