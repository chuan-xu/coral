use clap::Parser;

use crate::error::CoralRes;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(next_line_help = true)]
pub struct Cli {
    #[arg(long, help = "ca证书目录")]
    pub ca_dir: Option<String>,

    #[arg(long, help = "服务器证书")]
    pub certificate: String,

    #[arg(long, help = "服务器私钥")]
    pub private_key: String,

    #[arg(long, help = "服务的端口号")]
    pub port: u16,

    #[arg(long, help = "核数起始编号")]
    pub cpui: usize,

    #[arg(long, help = "runtime线程数")]
    pub nums: usize,

    #[arg(long, help = "多个server服务地址, 例如192.168.1.3:9001")]
    pub addresses: Vec<String>,

    #[arg(long, help = "日志文件保持路径")]
    pub log_dir: String,

    #[arg(long, help = "是否以debug模式启动")]
    pub debug: bool,
}

pub fn parse() -> CoralRes<Cli> {
    let args = Cli::parse();
    if !args.debug && args.log_dir.len() == 0 {
        return Err(crate::error::Error::MissingLogDir);
    }
    Ok(args)
}
