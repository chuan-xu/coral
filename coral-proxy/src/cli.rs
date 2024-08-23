use clap::Parser;

use crate::error::CoralRes;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(long, help = "server port")]
    pub port: u16,

    #[command(flatten)]
    pub comm_param: coral_util::cli::CommParam,

    #[command(flatten)]
    pub log_param: coral_log::LogParam,

    #[command(flatten)]
    pub runtime_param: coral_runtime::RuntimeParam,
}

impl Cli {
    pub(crate) fn init() -> CoralRes<Self> {
        let args = Cli::parse();
        args.comm_param.check()?;
        args.log_param.check()?;
        args.runtime_param.check()?;
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
