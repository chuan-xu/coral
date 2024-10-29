use bytes::Bytes;
use coral_runtime::spawn;
use log::error;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::cli::Conf;
use crate::error::CoralRes;
use coral_net::db;
use coral_net::error::Error as NetErr;
use futures::future::BoxFuture;

pub struct App<F> {
    conf: Conf,
    fns: Vec<F>,
    dbh: Option<BoxFuture<'static, Result<db::PgPool, NetErr>>>,
    rdh: Option<BoxFuture<'static, Result<db::RedisClient, NetErr>>>,
    h2_builder: Option<coral_net::server::ServerBuiler>,
    h3_builder: Option<coral_net::server::ServerBuiler>,
}

impl<F: FnOnce() -> ()> App<F> {
    pub async fn run(mut self) -> CoralRes<()> {
        while let Some(f) = self.fns.pop() {
            f();
        }
        let dbh = if let Some(dbh) = self.dbh.take() {
            Some(dbh.await?)
        } else {
            None
        };
        let rdh = if let Some(rdh) = self.rdh.take() {
            Some(rdh.await?)
        } else {
            None
        };
        if let Some(h2_builder) = self.h2_builder.take() {
            let dbhc = dbh.clone();
            let rdhc = rdh.clone();
            let map_req = move |mut req: hyper::Request<hyper::body::Incoming>,
                                router: axum::Router| {
                if let Some(h) = dbhc.clone() {
                    req.extensions_mut().insert(h);
                }
                if let Some(h) = rdhc.clone() {
                    req.extensions_mut().insert(h);
                }
                coral_net::hand::redirect_h2(req, router)
            };
            spawn(async {
                if let Err(err) = h2_builder.h2_server(Some(map_req)).await {
                    log::error!(e = format!("{:?}", err); "h2 server run error");
                }
            });
        }
        let h3_builder = self.h3_builder.take().unwrap();
        let mut transport_config = quinn_proto::TransportConfig::default();
        transport_config.max_idle_timeout(Some(quinn_proto::VarInt::from_u32(3600000).into()));
        let h3_server =
            h3_builder.h3_server(Some(Arc::new(transport_config)), move |mut req| {
                if let Some(h) = dbh.clone() {
                    req.extensions_mut().insert(h);
                }
                if let Some(h) = rdh.clone() {
                    req.extensions_mut().insert(h);
                }
                req
            })?;
        if self.conf.h3.server_conf.domain.is_some() && self.conf.h3.service_address.is_some() {
            let authorith = format!(
                "{}:{}",
                self.conf.h3.server_conf.domain.unwrap(),
                self.conf.h3.server_conf.port
            );
            report(
                h3_server.clone(),
                self.conf.h3.service_address.as_ref().unwrap(),
                authorith,
            )
            .await?;
        }
        h3_server.run_server().await?;
        Ok(())
    }
}

impl Conf {
    pub fn app(&self, router: axum::Router) -> CoralRes<App<impl FnOnce()>> {
        let mut fns = Vec::new();
        if let Some(f) = self.log_conf.set_traces() {
            fns.push(f);
        }
        Ok(App {
            conf: self.clone(),
            fns,
            dbh: self.dbh(),
            rdh: self.rdh(),
            h2_builder: Some(self.h2_builder(router.clone())?),
            h3_builder: Some(self.h3_server(router)?),
        })
    }

    fn dbh(&self) -> Option<futures::future::BoxFuture<'static, Result<db::PgPool, NetErr>>> {
        if let Some(v) = self.db.as_ref() {
            match v.postgres() {
                Ok(fut) => fut,
                Err(err) => {
                    log::error!(e = format!("{:?}", err); "failed to parse db conf");
                    std::panic!("failed to start!");
                }
            }
        } else {
            None
        }
    }

    fn rdh(&self) -> Option<futures::future::BoxFuture<'static, Result<db::RedisClient, NetErr>>> {
        if let Some(v) = self.redis.as_ref() {
            match v.client(None) {
                Ok(fut) => Some(fut),
                Err(err) => {
                    log::error!(e = format!("{:?}", err); "failed to parse redis conf");
                    std::panic!("failed to start!");
                }
            }
        } else {
            None
        }
    }

    fn h2_builder(&self, router: axum::Router) -> CoralRes<coral_net::server::ServerBuiler> {
        let addr_h2 = SocketAddr::new(
            std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            self.h2.server_conf.port,
        );
        let tls_conf = self.h2.tls_conf.server_conf()?;

        let h2_builder =
            coral_net::server::ServerBuiler::new(addr_h2, tls_conf).set_router(router.clone());
        Ok(h2_builder)
    }

    fn h3_server(&self, router: axum::Router) -> CoralRes<coral_net::server::ServerBuiler> {
        crate::hand::H3_PORT.set(self.h3.server_conf.port).unwrap();
        let addr_h3 = SocketAddr::new(
            std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            self.h3.server_conf.port,
        );
        let builder =
            coral_net::server::ServerBuiler::new(addr_h3, self.h3.tls_conf.server_conf()?)
                .set_router(router);
        Ok(builder)
    }
}

async fn report<F: Fn(hyper::Request<()>) -> hyper::Request<()> + Clone + Send + Sync + 'static>(
    h3_server: coral_net::server::H3Server<F>,
    service_address: &str,
    authority: String,
) -> CoralRes<()> {
    let (addr, domain) = coral_net::client::lookup_host(service_address).await?;
    let mut sender = h3_server.create_h3_client(addr, &domain, true).await?;
    let req = hyper::Request::builder()
        .method("POST")
        .uri(service_address)
        .body(())
        .map_err(|e| crate::error::Error::CoralNetErr(coral_net::error::Error::HttpInner(e)))?;
    let map_h3_err = |e| crate::error::Error::CoralNetErr(coral_net::error::Error::H3Err(e));
    let mut stream = sender.send_request(req).await.map_err(map_h3_err)?;
    stream.send_data(Bytes::from(authority)).await?;
    stream.finish().await.map_err(map_h3_err)?;
    let rsp = stream.recv_response().await.map_err(map_h3_err)?;
    if !rsp.status().is_success() {
        error!(
            "failed to report local to service with status: {}",
            rsp.status()
        );
    }
    Ok(())
}
