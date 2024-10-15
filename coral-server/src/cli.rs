use crate::error::CoralRes;
use clap::Parser;
use coral_conf::EnvAssignToml;
use coral_macro::EnvAssign;
use serde::Deserialize;
use std::io::Read;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(long, help = "toml config file path")]
    config: String,
}

#[derive(Deserialize, Debug, EnvAssign, Clone)]
pub(crate) struct H2Conf {
    pub(crate) server_conf: coral_net::server::ServerConf,
    pub(crate) tls_conf: coral_net::tls::TlsConf,
}

#[derive(Deserialize, Debug, EnvAssign, Clone)]
pub(crate) struct H3Conf {
    pub(crate) server_conf: coral_net::server::ServerConf,
    pub(crate) tls_conf: coral_net::tls::TlsConf,
    pub(crate) service_address: Option<String>,
}

#[derive(Deserialize, Debug, EnvAssign, Clone)]
pub struct AssetsConf {
    path: String,
    dir: String,
}

impl AssetsConf {
    pub fn service(&self) -> axum::Router {
        let serv = tower_http::services::fs::ServeDir::new(&self.dir)
            .precompressed_gzip()
            .precompressed_br()
            .precompressed_deflate()
            .precompressed_zstd();
        axum::Router::new().route_service(&self.path, serv)
    }
}

#[derive(Deserialize, Debug, EnvAssign, Clone)]
pub(crate) struct Conf {
    pub(crate) h2: H2Conf,
    pub(crate) h3: H3Conf,
    pub(crate) log_conf: coral_log::LogConf,
    pub(crate) rt_conf: coral_runtime::RuntimeConf,
    pub(crate) assets: Option<AssetsConf>,
    pub(crate) db: Option<coral_net::db::DbConf>,
    pub(crate) redis: Option<coral_net::db::RedisConf>,
}

impl Cli {
    pub(crate) fn init() -> CoralRes<Conf> {
        let args = Cli::parse();
        let mut file = std::fs::File::open(args.config)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        let mut conf: Conf = toml::from_str(&buf)?;
        conf.assign(Some("SERVER"))?;
        conf.h2.tls_conf.check()?;
        conf.h3.tls_conf.check()?;
        conf.log_conf.check()?;
        conf.rt_conf.check()?;
        Ok(conf)
    }
}
