use clap::Parser;

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

    #[arg(long, help = "service address: https://xxx.xxx.com:xx/xxx")]
    pub service_address: String,
}

impl Cli {
    pub(crate) fn init() -> CoralRes<Self> {
        let args = Cli::parse();
        args.tls_param.check()?;
        args.log_param.check()?;
        args.runtime_param.check()?;
        Ok(args)
    }
}
