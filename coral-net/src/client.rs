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

pub struct StatisticsGuard(pub(crate) Arc<AtomicU32>);

impl Drop for StatisticsGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

pub trait Statistics {
    fn usage_count(&self) -> (u32, u8);

    fn usage_add(&self) -> StatisticsGuard;

    fn is_valid(&self) -> bool {
        true
    }
}
/// use sample vector
pub struct VecClients<T, R, H> {
    inner: Arc<tokio::sync::RwLock<Vec<T>>>,
    phr: PhantomData<Arc<std::sync::Mutex<R>>>,
    phh: PhantomData<Arc<std::sync::Mutex<H>>>,
}

impl<T, R, H> Default for VecClients<T, R, H> {
    fn default() -> Self {
        Self {
            inner: Arc::new(tokio::sync::RwLock::new(vec![])),
            phr: PhantomData,
            phh: PhantomData,
        }
    }
}

impl<T, R, H> Clone for VecClients<T, R, H>
where T: Clone + Statistics
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            phr: self.phr.clone(),
            phh: self.phh.clone(),
        }
    }
}

impl<T, R, H> VecClients<T, R, H>
where
    T: Request<R, H> + Statistics + Clone + Send + Sync + 'static,
    R: Send + 'static,
    H: Send + 'static,
{
    async fn clean(self) {
        let mut pool = self.inner.write().await;
        pool.retain(|x| x.is_valid());
    }

    pub async fn load_balance(self: Self) -> CoralRes<Option<(T, crate::client::StatisticsGuard)>> {
        let pool = self.inner.read().await;
        let mut min = u32::MAX;
        let mut instance = None;
        for item in pool.iter() {
            let (count, state) = item.usage_count();
            if count < min && state == NORMAL {
                min = count;
                instance = Some((item.clone(), item.usage_add()));
            } else if state == CLOSED {
                let this = self.clone();
                tokio::spawn(this.clean());
            }
        }
        Ok(instance)
    }

    pub async fn add(self, conn: T) {
        let mut pool = self.inner.write().await;
        pool.push(conn);
    }
}
