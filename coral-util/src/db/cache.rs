//! some redis

use async_trait::async_trait;
use log::error;
use std::cell::RefCell;
use std::future::Future;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use crate::consts::{REDIS_KEY_DISCOVER, REDIS_KEY_NOTIFY};
use crate::error::{CoralRes, Error};

// redis TODO

/// mini-redis

// unsafe impl Send for MiniRedis {}
// unsafe impl Sync for MiniRedis {}

pub async fn discover<F, Fut, P>(
    addr: String,
    channels: Vec<String>,
    f: F,
    p: P,
    state: Arc<AtomicU8>,
) where
    F: Fn(Vec<String>, P) -> Fut,
    Fut: Future<Output = ()> + 'static,
    P: Clone,
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

// #[async_trait]
// impl SvcDiscover for MiniRedis {
//     async fn get_backends(&self) -> Vec<String> {
//         if let Ok(val) = self.client.borrow_mut().get(REDIS_KEY_DISCOVER).await {
//             if let Some(val) = val {
//                 return val
//                     .split(|k| *k == 44)
//                     .filter_map(|item| std::str::from_utf8(item).ok())
//                     .map(|x| x.to_owned())
//                     .collect();
//             }
//         }
//         vec![]
//     }

//     async fn listen_update(&self) -> bool {
//         if let Some(subscriber) = self.subscriber.as_mut() {
//             if let Ok(msg) = subscriber.next_message().await {
//                 return msg.is_some();
//             }
//         }
//         false
//     }
// }

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
