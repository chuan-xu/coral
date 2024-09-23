#![allow(unused)]
use std::marker::PhantomData;
use std::sync::Arc;

use coral_runtime::tokio::net::ToSocketAddrs;
use log::error;
use log::info;
use log::warn;

use crate::error::CoralRes;
use crate::error::Error;

pub static REDIS_KEY_NOTIFY: &'static str = "svc_update";

pub static REDIS_KEY_DISCOVER: &'static str = "svc_endpoints";

/// 127.0.0.1:9001,127.0.0.1:9002
static ENDPOINTS_SPLIT_TAG: u8 = 44;

#[derive(Debug)]
pub struct DiscoverParam {
    pub discover_uri: Option<String>,
}

#[async_trait::async_trait]
pub trait Discover<F, P>
where
    F: Fn(Vec<String>, P) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + 'static,
    P: Clone + Send + 'static,
{
    async fn discover(&mut self, f: F, param: P);
}

/// temporary solution
pub struct MiniRedis {
    client: mini_redis::Client,
    subscriber: Option<mini_redis::clients::Subscriber>,
}

#[async_trait::async_trait]
impl<F, P> Discover<F, P> for MiniRedis
where
    F: Fn(Vec<String>, P) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + 'static,
    P: Clone + Send + 'static,
{
    async fn discover(&mut self, f: F, param: P) {
        if self.subscriber.is_none() {
            return;
        }
        loop {
            match self.get(REDIS_KEY_DISCOVER).await {
                Ok(Some(val)) => {
                    let endpoints: Vec<String> = val
                        .split(|x| *x == ENDPOINTS_SPLIT_TAG)
                        .filter_map(|x| std::str::from_utf8(x).ok())
                        .map(|x| x.to_owned())
                        .collect();
                    if !endpoints.is_empty() {
                        f(endpoints, param.clone()).await;
                    }
                }
                Ok(None) => {
                    warn!("{} is empty", REDIS_KEY_DISCOVER);
                }
                Err(err) => {
                    error!(e = format!("{:?}", err); "failed to get {}", REDIS_KEY_DISCOVER);
                }
            }
            if let Some(subscriber) = self.subscriber.as_mut() {
                match subscriber.next_message().await {
                    Ok(Some(_)) => {
                        info!("subscriber receive message");
                    }
                    Ok(None) => {
                        warn!("subscriber message is empty");
                    }
                    Err(err) => {
                        error!(e = format!("{:?}", err); "failed to subscriber get next message");
                    }
                }
            }
        }
    }
}

impl MiniRedis {
    pub async fn new<A>(addr: A) -> CoralRes<Self>
    where
        A: ToSocketAddrs,
    {
        match mini_redis::Client::connect(addr).await {
            Ok(client) => Ok(Self {
                client,
                subscriber: None,
            }),
            Err(err) => {
                error!(e = format!("{:?}", err); "failed to connect mini redis");
                Err(Error::DiscoverConnErr)
            }
        }
    }

    pub async fn set_subscriber<A: ToSocketAddrs>(
        &mut self,
        addr: A,
        channels: Vec<String>,
    ) -> CoralRes<()> {
        if self.subscriber.is_none() {
            match mini_redis::Client::connect(&addr).await {
                Ok(client) => match client.subscribe(channels).await {
                    Ok(subscriber) => {
                        self.subscriber = Some(subscriber);
                        Ok(())
                    }
                    Err(err) => {
                        error!(e = format!("{:?}", err); "failed to mini_redis client subscribe");
                        Err(Error::DiscoverSubscribeErr)
                    }
                },
                Err(err) => {
                    error!(e = format!("{:?}", err); "failed to mini_redis client connect");
                    Err(Error::DiscoverConnErr)
                }
            }
        } else {
            Ok(())
        }
    }

    pub async fn get(&mut self, key: &str) -> CoralRes<Option<bytes::Bytes>> {
        match self.client.get(key).await {
            Ok(val) => Ok(val),
            Err(err) => {
                error!(e = format!("{:?}", err); "failed to get mini redis value");
                Err(Error::DiscoverGetErr)
            }
        }
    }

    pub async fn set(&mut self, key: &str, value: bytes::Bytes) -> CoralRes<()> {
        match self.client.set(key, value).await {
            Ok(_) => Ok(()),
            Err(err) => {
                error!(e = format!("{:?}", err); "failed to set mini redis value");
                Err(Error::DiscoverSetErr)
            }
        }
    }

    pub async fn publish(&mut self, channel: &str, data: bytes::Bytes) -> CoralRes<()> {
        match self.client.publish(channel, data).await {
            Ok(_) => Ok(()),
            Err(err) => {
                error!(e = format!("{:?}", err); "failed to publish by mini redis client");
                Err(Error::DiscoverPublishErr)
            }
        }
    }
}
