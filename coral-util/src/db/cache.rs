//! some redis

use std::future::Future;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use log::error;
use tokio::net::ToSocketAddrs;

use crate::consts::REDIS_KEY_DISCOVER;
use crate::error::CoralRes;
use crate::error::Error;

// redis TODO

pub struct MiniRedis {
    inner: mini_redis::Client,
}

impl MiniRedis {
    pub async fn new<T>(addr: T) -> CoralRes<Self>
    where T: ToSocketAddrs {
        match mini_redis::Client::connect(addr).await {
            Ok(client) => Ok(Self { inner: client }),
            Err(err) => {
                let e_str = err.to_string();
                error!(e = e_str.as_str(); "failed to connect mini redis");
                Err(Error::CacheCreateErr)
            }
        }
    }

    pub async fn get(&mut self, key: &str) -> CoralRes<Option<bytes::Bytes>> {
        match self.inner.get(key).await {
            Ok(val) => Ok(val),
            Err(err) => {
                let e_str = err.to_string();
                error!(e = e_str.as_str(); "");
                Err(Error::CacheGetErr)
            }
        }
    }

    pub async fn set(&mut self, key: &str, value: bytes::Bytes) -> CoralRes<()> {
        match self.inner.set(key, value).await {
            Ok(_) => Ok(()),
            Err(err) => {
                let e_str = err.to_string();
                error!(e = e_str.as_str(); "failed to set mini redis value");
                Err(Error::CacheSetErr)
            }
        }
    }

    pub async fn publish(&mut self, channel: &str, data: bytes::Bytes) -> CoralRes<()> {
        match self.inner.publish(channel, data).await {
            Ok(_) => Ok(()),
            Err(err) => {
                let e_str = err.to_string();
                error!(e = e_str.as_str(); "failed to publish by mini redis client");
                Err(Error::CachePublishErr)
            }
        }
    }
}

pub async fn discover<F, Fut, P, S>(
    addr: S,
    channels: Vec<String>,
    f: F,
    p: P,
    state: Arc<AtomicU8>,
) where
    F: Fn(Vec<String>, P) -> Fut,
    Fut: Future<Output = ()> + 'static,
    P: Clone,
    S: ToSocketAddrs,
{
    let client = mini_redis::Client::connect(&addr).await;
    if let Err(err) = client {
        let e_str = err.to_string();
        error!(e = e_str.as_str(); "failed to mini_redis client connect");
        state.store(1, Ordering::Release);
        return;
    }
    let mut client = client.unwrap();
    let subscriber = mini_redis::Client::connect(&addr).await;
    if let Err(err) = subscriber {
        let e_str = err.to_string();
        error!(e = e_str.as_str(); "failed to mini_redis client connect");
        state.store(1, Ordering::Release);
        return;
    }
    let subscriber = subscriber.unwrap().subscribe(channels).await;
    if let Err(err) = subscriber {
        let e_str = err.to_string();
        error!(e = e_str.as_str(); "failed to mini_redis client subscribe");
        state.store(1, Ordering::Release);
        return;
    }
    state.store(2, Ordering::Release);
    let mut subscriber = subscriber.unwrap();
    loop {
        match subscriber.next_message().await {
            Ok(data) => {
                if data.is_some() {
                    match client.get(REDIS_KEY_DISCOVER).await {
                        Ok(val) => {
                            if let Some(val) = val {
                                f(
                                    val.split(|k| *k == 44)
                                        .filter_map(|k| std::str::from_utf8(k).ok())
                                        .map(|k| k.to_owned())
                                        .collect(),
                                    p.clone(),
                                )
                                .await;
                            }
                        }
                        Err(err) => {
                            let e_str = err.to_string();
                            error!(e = e_str.as_str(); "failed to get {}", REDIS_KEY_DISCOVER);
                        }
                    }
                }
            }
            Err(err) => {
                let e_str = err.to_string();
                error!(e = e_str.as_str(); "failed to subscribe get next_message");
            }
        }
    }
}

#[allow(unused)]
mod seal {
    pub trait Seal {}
}

#[cfg(test)]
mod test {
    #[test]
    #[ignore = "manual"]
    fn mini_redis_subscribe() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let f = async {
            let client = mini_redis::Client::connect("127.0.0.1:6379").await.unwrap();
            let sub_key = vec![String::from("name"), String::from("age")];
            let mut sub = client.subscribe(sub_key).await.unwrap();
            let msg = sub.next_message().await.unwrap();
            println!("{:?}", msg);
        };
        rt.block_on(f);
    }
}
