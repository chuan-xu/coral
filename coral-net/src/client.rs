use std::marker::PhantomData;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use coral_runtime::tokio;

use crate::error::CoralRes;

/// client is normal
pub static NORMAL: u8 = 0;

/// server has reject
pub static REJECT: u8 = 1;

/// server is closed
pub static CLOSED: u8 = 2;

/// handle is cleaning
pub static CLEANING: u8 = 3;

/// handle has cleaned
pub static CLEANED: u8 = 4;

#[async_trait::async_trait]
pub trait Request<R, H> {
    async fn send(&mut self, req: hyper::Request<R>) -> CoralRes<hyper::Response<H>>;
}

#[async_trait::async_trait]
pub trait Pool {
    // fn add(&mut self) -> CoralRes<()>;
    type Client;

    async fn load_balance(self: Arc<Self>) -> CoralRes<Option<(Self::Client, StatisticsGuard)>>;
}

pub struct StatisticsGuard(pub(crate) Arc<AtomicU32>);

impl Drop for StatisticsGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

pub trait statistics {
    fn usage_count(&self) -> (u32, u8);

    fn usage_add(&self) -> StatisticsGuard;
}
/// use sample vector
struct VecClients<T, R, H> {
    inner: tokio::sync::RwLock<Vec<T>>,
    phr: PhantomData<R>,
    phh: PhantomData<H>,
}

#[async_trait::async_trait]
impl<T, R, H> crate::client::Pool for VecClients<T, R, H>
where
    T: crate::client::Request<R, H> + crate::client::statistics + Clone + Send + Sync,
    R: Send + Sync,
    H: Send + Sync,
{
    type Client = T;

    async fn load_balance(
        self: Arc<Self>,
    ) -> CoralRes<Option<(Self::Client, crate::client::StatisticsGuard)>> {
        let pool = self.inner.read().await;
        let mut min = u32::MAX;
        let mut instance = None;
        for item in pool.iter() {
            let (count, state) = item.usage_count();
            if count < min {
                min = count;
                instance = Some((item.clone(), item.usage_add()));
            }
        }
        Ok(instance)
    }
}
