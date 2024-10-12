use std::str::FromStr;

use coral_conf::EnvAssignToml;
use coral_macro::EnvAssign;
use futures::FutureExt;
use redis::{
    aio::{ConnectionLike, ConnectionManager, MultiplexedConnection},
    cluster_async::ClusterConnection,
};
use serde::Deserialize;
use sqlx::ConnectOptions;

use crate::error::CoralRes;

#[derive(Deserialize, EnvAssign, Debug)]
pub(crate) struct LogSettings {
    pub(crate) statements_level: String,
    pub(crate) slow_statements_level: String,
    pub(crate) slow_statements_duration: u64,
}

#[derive(Deserialize, EnvAssign, Debug)]
pub(crate) struct PgConnectOptions {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) socket: Option<String>,
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    pub(crate) database: Option<String>,
    pub(crate) ssl_mode: String,
    pub(crate) ssl_root_cert: Option<String>,
    pub(crate) ssl_client_cert: Option<String>,
    pub(crate) ssl_client_key: Option<String>,
    pub(crate) statement_cache_capacity: Option<usize>,
    pub(crate) application_name: Option<String>,
    pub(crate) log_settings: Option<LogSettings>,
}

impl TryFrom<&PgConnectOptions> for sqlx::postgres::PgConnectOptions {
    type Error = crate::error::Error;

    fn try_from(value: &PgConnectOptions) -> Result<Self, Self::Error> {
        let mut this = sqlx::postgres::PgConnectOptions::new()
            .host(&value.host)
            .port(value.port)
            .username(&value.username)
            .ssl_mode(sqlx::postgres::PgSslMode::from_str(&value.ssl_mode)?);
        if let Some(v) = value.socket.as_ref() {
            this = this.socket(v);
        }
        if let Some(v) = value.password.as_ref() {
            this = this.password(v);
        }
        if let Some(v) = value.database.as_ref() {
            this = this.database(v);
        }
        if let Some(v) = value.ssl_root_cert.as_ref() {
            this = this.ssl_root_cert(v);
        }
        if let Some(v) = value.ssl_client_cert.as_ref() {
            this = this.ssl_client_cert(v);
        }
        if let Some(v) = value.ssl_client_key.as_ref() {
            this = this.ssl_client_key(v);
        }
        if let Some(v) = value.statement_cache_capacity {
            this = this.statement_cache_capacity(v);
        }
        if let Some(v) = value.application_name.as_ref() {
            this = this.application_name(v);
        }
        if let Some(v) = value.log_settings.as_ref() {
            this = this
                .log_statements(log::LevelFilter::from_str(&v.statements_level)?)
                .log_slow_statements(
                    log::LevelFilter::from_str(&v.slow_statements_level)?,
                    std::time::Duration::from_secs(v.slow_statements_duration),
                );
        }
        Ok(this)
    }
}

#[derive(Deserialize, EnvAssign, Debug)]
pub(crate) struct PoolOptions {
    pub(crate) test_before_acquire: Option<bool>,
    pub(crate) max_connections: Option<u32>,
    pub(crate) acquire_time_level: Option<String>,
    pub(crate) acquire_slow_level: Option<String>,
    pub(crate) acquire_slow_threshold: Option<u64>,
    pub(crate) acquire_timeout: Option<u64>,
    pub(crate) min_connections: Option<u32>,
    pub(crate) max_lifetime: Option<u64>,
    pub(crate) idle_timeout: Option<u64>,
    pub(crate) fair: Option<bool>,
}

impl<DB: sqlx::Database> TryFrom<&PoolOptions> for sqlx::pool::PoolOptions<DB> {
    type Error = crate::error::Error;

    fn try_from(value: &PoolOptions) -> Result<Self, Self::Error> {
        let mut this = sqlx::pool::PoolOptions::<DB>::new();

        if let Some(v) = value.test_before_acquire {
            this = this.test_before_acquire(v);
        }
        if let Some(v) = value.max_connections {
            this = this.max_connections(v);
        }
        if let Some(v) = value.acquire_time_level.as_ref() {
            this = this.acquire_time_level(log::LevelFilter::from_str(v)?);
        }
        if let Some(v) = value.acquire_slow_level.as_ref() {
            this = this.acquire_slow_level(log::LevelFilter::from_str(v)?);
        }
        if let Some(v) = value.acquire_slow_threshold {
            this = this.acquire_slow_threshold(std::time::Duration::from_secs(v));
        }
        if let Some(v) = value.acquire_timeout {
            this = this.acquire_timeout(std::time::Duration::from_secs(v));
        }
        if let Some(v) = value.min_connections {
            this = this.min_connections(v);
        }
        if let Some(v) = value.max_lifetime {
            this = this.max_lifetime(std::time::Duration::from_secs(v));
        }
        if let Some(v) = value.idle_timeout {
            this = this.idle_timeout(std::time::Duration::from_secs(v));
        }
        if let Some(v) = value.fair {
            this = this.__fair(v);
        }
        Ok(this)
    }
}

#[derive(Deserialize, EnvAssign, Debug)]
pub struct DbConf {
    pool: Option<PoolOptions>,
    postgres: Option<PgConnectOptions>,
}

impl DbConf {
    pub async fn postgres(&self) -> CoralRes<Option<sqlx::Pool<sqlx::Postgres>>> {
        if let Some(pool_options) = self.pool.as_ref() {
            if let Some(pg_conn_options) = self.postgres.as_ref() {
                return Ok(Some(
                    sqlx::pool::PoolOptions::<sqlx::Postgres>::try_from(pool_options)?
                        .connect_with(sqlx::postgres::PgConnectOptions::try_from(pg_conn_options)?)
                        .await?,
                ));
            }
        }
        Ok(None)
    }
}

#[derive(Deserialize, EnvAssign, Debug)]
enum RedisConf {
    // TODO:
    Single(String),
}

#[derive(Clone)]
pub enum RedisClient {
    MultiConn(MultiplexedConnection),
    ManagerConn(ConnectionManager),
    ClusterConn(ClusterConnection),
}

impl ConnectionLike for RedisClient {
    fn req_packed_command<'a>(
        &'a mut self,
        cmd: &'a redis::Cmd,
    ) -> redis::RedisFuture<'a, redis::Value> {
        match self {
            RedisClient::MultiConn(t) => t.req_packed_command(cmd),
            RedisClient::ManagerConn(t) => t.req_packed_command(cmd),
            RedisClient::ClusterConn(t) => t.req_packed_command(cmd),
        }
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a redis::Pipeline,
        offset: usize,
        count: usize,
    ) -> redis::RedisFuture<'a, Vec<redis::Value>> {
        match self {
            RedisClient::MultiConn(t) => t.req_packed_commands(cmd, offset, count),
            RedisClient::ManagerConn(t) => t.req_packed_commands(cmd, offset, count),
            RedisClient::ClusterConn(t) => t.req_packed_commands(cmd, offset, count),
        }
    }

    fn get_db(&self) -> i64 {
        match self {
            RedisClient::MultiConn(t) => t.get_db(),
            RedisClient::ManagerConn(t) => t.get_db(),
            RedisClient::ClusterConn(t) => t.get_db(),
        }
    }
}

async fn conn() {
    // let conn_addr = redis::ConnectionAddr::TcpTls { host: String::from("111.229.180.248"), port: 6379, insecure: , tls_params:  }
    let conn_addr = redis::ConnectionAddr::Tcp(String::from("111.229.180.248"), 6379);
    // redis::Client::build_with_tls(, )
    let redis_info = redis::RedisConnectionInfo {
        db: 0,
        username: None,
        password: Some(String::from("112233zts")),
        protocol: redis::ProtocolVersion::RESP3,
    };
    let conn_info = redis::ConnectionInfo {
        addr: conn_addr,
        redis: redis_info,
    };
    let client = redis::Client::open(conn_info).unwrap();
    let mut mconn = client.get_multiplexed_tokio_connection().await.unwrap();
    let res = redis::cmd("GET")
        .arg("name")
        .query_async::<String>(&mut mconn)
        .await
        .unwrap();
    println!("{:?}", res);
    // redis::cluster::ClusterClient
    // redis::Client::build_with_tls(, )

    // redis::Client::build_with_tls(, )
    // redis::ClientTlsConfig::

    // config = redis::aio::ConnectionManagerConfig::new().set_push_sender()
    // client.get_connection_manager_with_config()
    // let manager = client.get_connection_manager().await.unwrap();
    // redis::Client::build_with_tls(, )
}
