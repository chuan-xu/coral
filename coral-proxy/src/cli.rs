use std::io::Read;

use clap::Parser;
use serde::Deserialize;

use crate::error::CoralRes;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub server_param: coral_net::server::ServerParam,

    #[command(flatten)]
    pub tls_param: coral_net::tls::TlsParam,

    #[command(flatten)]
    pub log_param: coral_log::LogParam,

    #[command(flatten)]
    pub runtime_param: coral_runtime::RuntimeParam,

    #[command(flatten)]
    pub discover_param: coral_net::discover::DiscoverParam,

    #[arg(long, help = "the uri of discover service")]
    pub conn_conf: Option<String>,
}

impl Cli {
    pub(crate) fn init() -> CoralRes<Self> {
        let args = Cli::parse();
        args.tls_param.check()?;
        args.log_param.check()?;
        args.runtime_param.check()?;
        Ok(args)
    }

    pub(crate) fn get_conn(&self) -> CoralRes<Vec<ConnConf>> {
        if let Some(path) = self.conn_conf.as_ref() {
            let mut fd = std::fs::File::open(path)?;
            let mut buf = Vec::new();
            fd.read_to_end(&mut buf)?;
            let conn_conf: Vec<ConnConf> = serde_json::from_slice(buf.as_slice())?;
            return Ok(conn_conf);
        }
        Ok(vec![])
    }
}

#[derive(Deserialize, Debug)]
pub struct ConnConf {
    pub ip: String,
    pub port: u16,
    pub domain: String,
    pub ca: Option<String>,
    pub cert: String,
    pub key: String,
}
