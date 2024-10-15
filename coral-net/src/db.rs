use std::{io::Read, str::FromStr};

use coral_conf::EnvAssignToml;
use coral_macro::EnvAssign;
use redis::{
    aio::{ConnectionLike, ConnectionManager, MultiplexedConnection},
    cluster_async::ClusterConnection,
};
use serde::Deserialize;
use sqlx::ConnectOptions;

use crate::error::CoralRes;

#[derive(Deserialize, EnvAssign, Debug, Clone)]
pub(crate) struct LogSettings {
    pub(crate) statements_level: String,
    pub(crate) slow_statements_level: String,
    pub(crate) slow_statements_duration: u64,
}

#[derive(Deserialize, EnvAssign, Debug, Clone)]
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

#[derive(Deserialize, EnvAssign, Debug, Clone)]
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

#[derive(Deserialize, EnvAssign, Debug, Clone)]
pub struct DbConf {
    pool: Option<PoolOptions>,
    postgres: Option<PgConnectOptions>,
}

pub type PgPool = sqlx::Pool<sqlx::Postgres>;

impl DbConf {
    pub async fn postgres(&self) -> CoralRes<Option<PgPool>> {
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

#[derive(Deserialize, EnvAssign, Debug, Clone)]
struct RedisTls {
    root_cert_store: Option<String>,
    client_cert: Option<String>,
    client_key: Option<String>,
}

impl TryFrom<&RedisTls> for redis::TlsCertificates {
    type Error = crate::error::Error;

    fn try_from(value: &RedisTls) -> Result<Self, Self::Error> {
        let mut this = Self {
            client_tls: None,
            root_cert: None,
        };
        if value.client_cert.is_some() && value.client_key.is_some() {
            let mut cert_fd = std::fs::File::open(value.client_cert.as_ref().unwrap())?;
            let mut key_fd = std::fs::File::open(value.client_key.as_ref().unwrap())?;
            let mut cert_buf = Vec::new();
            let mut key_buf = Vec::new();
            cert_fd.read_to_end(&mut cert_buf)?;
            key_fd.read_to_end(&mut key_buf)?;
            this.client_tls = Some(redis::ClientTlsConfig {
                client_cert: cert_buf,
                client_key: key_buf,
            });
        }
        if let Some(v) = value.root_cert_store.as_ref() {
            let mut fd = std::fs::File::open(v)?;
            let mut buf = Vec::new();
            fd.read_to_end(&mut buf)?;
            this.root_cert = Some(buf);
        }
        Ok(this)
    }
}

#[derive(Deserialize, EnvAssign, Debug, Clone)]
pub struct RedisSingle {
    host: String,
    port: u16,
    insecure: bool,
    tls_params: Option<RedisTls>,
    db: Option<i64>,
    username: Option<String>,
    password: Option<String>,
    protocol: Option<u16>,
    config: RedisSingleConf,
}

#[derive(Deserialize, EnvAssign, Debug, Clone)]
enum RedisSingleConf {
    Manager(RedisConnManagerConf), // TODO: Multi
}

#[derive(Deserialize, EnvAssign, Debug, Clone)]
struct RedisConnManagerConf {
    exponent_base: Option<u64>,
    /// A multiplicative factor that will be applied to the retry delay.
    ///
    /// For example, using a factor of `1000` will make each delay in units of seconds.
    factor: Option<u64>,
    /// number_of_retries times, with an exponentially increasing delay
    number_of_retries: Option<usize>,
    /// Apply a maximum delay between connection attempts. The delay between attempts won't be longer than max_delay milliseconds.
    max_delay: Option<u64>,
    /// The new connection will time out operations after `response_timeout` has passed.
    response_timeout: Option<u64>,
    /// Each connection attempt to the server will time out after `connection_timeout`.
    connection_timeout: Option<u64>,
}

impl From<&RedisConnManagerConf> for redis::aio::ConnectionManagerConfig {
    fn from(value: &RedisConnManagerConf) -> Self {
        let mut this = Self::default();
        if let Some(v) = value.exponent_base {
            this = this.set_exponent_base(v);
        }
        if let Some(v) = value.factor {
            this = this.set_factor(v);
        }
        if let Some(v) = value.number_of_retries {
            this = this.set_number_of_retries(v);
        }
        if let Some(v) = value.max_delay {
            this = this.set_max_delay(v);
        }
        if let Some(v) = value.response_timeout {
            this = this.set_response_timeout(std::time::Duration::from_secs(v));
        }
        if let Some(v) = value.connection_timeout {
            this = this.set_connection_timeout(std::time::Duration::from_secs(v));
        }
        this
    }
}

impl From<&RedisSingle> for redis::RedisConnectionInfo {
    fn from(value: &RedisSingle) -> Self {
        let mut this = Self::default();
        if let Some(v) = value.db {
            this.db = v;
        }
        if let Some(v) = value.username.as_ref() {
            this.username = Some(v.to_owned());
        }
        if let Some(v) = value.password.as_ref() {
            this.password = Some(v.to_owned());
        }
        this.protocol = value.protocol.map_or(redis::ProtocolVersion::RESP2, |x| {
            if x == 1 {
                redis::ProtocolVersion::RESP3
            } else {
                redis::ProtocolVersion::RESP2
            }
        });
        this
    }
}

#[derive(Deserialize, EnvAssign, Debug, Clone)]
struct RedisRetryParams {
    number_of_retries: u32,
    max_wait_time: u64,
    min_wait_time: u64,
    exponent_base: u64,
    factor: u64,
}

#[derive(Deserialize, EnvAssign, Debug, Clone)]
pub struct RedisCluster {
    password: Option<String>,
    username: Option<String>,
    read_from_replicas: Option<bool>,
    insecure: bool,
    retry_params: Option<RedisRetryParams>,
    tls_params: Option<RedisTls>,
    connection_timeout: Option<u64>,
    response_timeout: Option<u64>,
    protocol: Option<u16>,
}

#[derive(Deserialize, EnvAssign, Debug, Clone)]
pub enum RedisConf {
    Single(RedisSingle),
    Cluster(RedisCluster),
}

pub type RedisAsyncPushSender = coral_runtime::tokio::sync::mpsc::UnboundedSender<redis::PushInfo>;

impl RedisConf {
    pub async fn client(&self, push_sender: Option<RedisAsyncPushSender>) -> CoralRes<RedisClient> {
        match self {
            RedisConf::Single(single) => {
                let info = redis::RedisConnectionInfo::from(single);
                let client = match single.tls_params.as_ref() {
                    Some(tls) => {
                        let addr = redis::ConnectionAddr::TcpTls {
                            host: single.host.clone(),
                            port: single.port,
                            insecure: single.insecure,
                            tls_params: None,
                        };
                        redis::Client::build_with_tls(
                            redis::ConnectionInfo { addr, redis: info },
                            redis::TlsCertificates::try_from(tls)?,
                        )?
                    }
                    None => {
                        let addr = redis::ConnectionAddr::Tcp(single.host.clone(), single.port);
                        redis::Client::open(redis::ConnectionInfo { addr, redis: info })?
                    }
                };
                match &single.config {
                    RedisSingleConf::Manager(conf) => {
                        let mut conn_conf = redis::aio::ConnectionManagerConfig::from(conf);
                        if let Some(sender) = push_sender {
                            conn_conf = conn_conf.set_push_sender(sender);
                        }
                        let rc = client.get_connection_manager_with_config(conn_conf).await?;
                        Ok(RedisClient::ManagerConn(rc))
                    }
                }
                // client.get_multiplexed_async_connection_with_config()
            }
            RedisConf::Cluster(_) => todo!(),
        }
    }
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
