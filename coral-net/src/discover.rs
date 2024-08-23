use std::{marker::PhantomData, sync::Arc};

use clap::Args;
use coral_runtime::tokio::net::ToSocketAddrs;
use log::error;

use crate::{
    error::{CoralRes, Error},
    http::HttpSendPool,
};

static REDIS_KEY_NOTIFY: &'static str = "svc_update";

static REDIS_KEY_DISCOVER: &'static str = "svc_endpoints";

#[derive(Args, Debug)]
pub struct DiscoverParam {
    #[arg(long, help = "the uri of discover service")]
    discover_uri: Option<String>,
}

pub trait Discover {
    type pool;

    fn discover(&mut self);
}

/// temporary solution
pub struct MiniRedis<T> {
    client: mini_redis::Client,
    subscriber: Option<mini_redis::clients::Subscriber>,
    _pd: PhantomData<Arc<T>>,
}

impl<T> Discover for MiniRedis<T> {
    type pool = HttpSendPool<T>;

    fn discover(&mut self) {}
}

impl<T> MiniRedis<T> {
    pub async fn new<A>(addr: A) -> CoralRes<Self>
    where
        A: ToSocketAddrs,
    {
        match mini_redis::Client::connect(addr).await {
            Ok(client) => Ok(Self {
                client,
                subscriber: None,
                _pd: PhantomData,
            }),
            Err(err) => {
                error!(e = err.to_string(); "failed to connect mini redis");
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
                        error!(e = err.to_string(); "failed to mini_redis client subscribe");
                        Err(Error::DiscoverSubscribeErr)
                    }
                },
                Err(err) => {
                    error!(e = err.to_string(); "failed to mini_redis client connect");
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
                error!(e = err.to_string(); "failed to get mini redis value");
                Err(Error::DiscoverGetErr)
            }
        }
    }

    pub async fn set(&mut self, key: &str, value: bytes::Bytes) -> CoralRes<()> {
        match self.client.set(key, value).await {
            Ok(_) => Ok(()),
            Err(err) => {
                error!(e = err.to_string(); "failed to set mini redis value");
                Err(Error::DiscoverSetErr)
            }
        }
    }

    pub async fn publish(&mut self, channel: &str, data: bytes::Bytes) -> CoralRes<()> {
        match self.client.publish(channel, data).await {
            Ok(_) => Ok(()),
            Err(err) => {
                error!(e = err.to_string(); "failed to publish by mini redis client");
                Err(Error::DiscoverPublishErr)
            }
        }
    }
}
