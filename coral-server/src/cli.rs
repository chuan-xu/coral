use clap::Parser;

use crate::error::CoralRes;

#[derive(Parser)]
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
        args.log_param.check()?;
        args.runtime_param.check()?;
        Ok(args)
    }

    // TODO
    // pub(crate) fn get_rotation(&self) -> CoralRes<coral_log::Rotation> {
    //     let rotation = self
    //         .log_rotation
    //         .as_ref()
    //         .ok_or(Error::MissingLogRotation)?;
    //     match rotation.as_str() {
    //         "min" => Ok(coral_log::Rotation::MINUTELY),
    //         "hour" => Ok(coral_log::Rotation::HOURLY),
    //         "day" => Ok(coral_log::Rotation::DAILY),
    //         _ => Ok(coral_log::Rotation::NEVER),
    //     }
    // }
}
