use clap::Args;

use crate::error::CoralRes;
use crate::error::Error;

#[derive(Args, Debug)]
pub struct CommParam {
    #[arg(long, help = "address to connect cache")]
    pub cache_addr: Option<String>,

    #[arg(long, help = "ca directory")]
    pub ca_dir: Option<String>,

    #[arg(long, help = "server certificate")]
    pub certificate: String,

    #[arg(long, help = "server private")]
    pub private_key: String,
}

impl CommParam {
    pub fn check(&self) -> CoralRes<()> {
        if let Some(dir) = self.ca_dir.as_ref() {
            if !std::fs::metadata(dir)?.is_dir() {
                return Err(Error::InvalidCa);
            }
        }
        Ok(())
    }
}
