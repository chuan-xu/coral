use clap::Parser;
use coral_log::Param;

use crate::error::CoralRes;
use crate::error::Error;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(next_line_help = true)]
pub struct Cli {
    #[arg(long, help = "ca directory")]
    pub ca_dir: Option<String>,

    #[arg(long, help = "server certificate")]
    pub certificate: String,

    #[arg(long, help = "server private")]
    pub private_key: String,

    #[arg(long, help = "server port")]
    pub port: u16,

    #[arg(long, help = "start number of cpu cores")]
    pub cpui: usize,

    #[arg(long, help = "number of runtime")]
    pub nums: usize,

    #[arg(long, help = "multiple backend address, exp 192.168.1.3:9001")]
    pub addresses: Vec<String>,

    #[command(flatten)]
    param: Param,
}

impl Cli {
    pub(crate) fn init() -> CoralRes<Self> {
        let args = Cli::parse();
        if let Some(dir) = args.ca_dir.as_ref() {
            if !std::fs::metadata(dir)?.is_dir() {
                return Err(Error::InvalidCa);
            }
        }
        args.param.check()?;
        Ok(args)
    }

    // TODO
    // pub(crate) fn get_rotation(&self) -> CoralRes<logs::Rotation> {
    //     let rotation = self
    //         .log_rotation
    //         .as_ref()
    //         .ok_or(Error::MissingLogRotation)?;
    //     match rotation.as_str() {
    //         "min" => Ok(logs::Rotation::MINUTELY),
    //         "hour" => Ok(logs::Rotation::HOURLY),
    //         "day" => Ok(logs::Rotation::DAILY),
    //         _ => Ok(logs::Rotation::NEVER),
    //     }
    // }
}
