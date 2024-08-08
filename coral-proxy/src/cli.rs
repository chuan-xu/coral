use clap::Parser;

use crate::error::{CoralRes, Error};

#[derive(Parser, Debug)]
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
    pub log_dir: Option<String>,

    #[arg(long, help = "日志分割周期")]
    pub log_rotation: Option<String>,

    #[arg(long, help = "是否以debug模式启动")]
    pub debug: bool,
}

impl Cli {
    pub(crate) fn init() -> CoralRes<Self> {
        let args = Cli::parse();
        if !args.debug && args.log_dir.is_none() {
            return Err(Error::MissingLogDir);
        }
        if let Some(dir) = args.ca_dir.as_ref() {
            if !std::fs::metadata(dir)?.is_dir() {
                return Err(Error::InvalidCa);
            }
        }
        if let Some(dir) = args.log_dir.as_ref() {
            if !std::fs::metadata(dir)?.is_dir() {
                return Err(Error::InvalidLogDir);
            }
        }
        Ok(args)
    }

    pub(crate) fn get_rotation(&self) -> CoralRes<coral_log::Rotation> {
        let rotation = self
            .log_rotation
            .as_ref()
            .ok_or(Error::MissingLogRotation)?;
        match rotation.as_str() {
            "min" => Ok(coral_log::Rotation::MINUTELY),
            "hour" => Ok(coral_log::Rotation::HOURLY),
            "day" => Ok(coral_log::Rotation::DAILY),
            _ => Ok(coral_log::Rotation::NEVER),
        }
    }
}
