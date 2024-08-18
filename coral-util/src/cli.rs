use clap::Args;

#[derive(Args, Debug)]
pub struct CommParam {
    #[arg(long, help = "address to connect cache")]
    pub cache_addr: Option<String>,
}
