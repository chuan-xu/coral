use clap::Args;

use crate::error::CoralRes;
use crate::error::Error;

#[derive(Args, Debug)]
pub struct RuntimeParam {
    #[arg(long, help = "start number of cpu cores")]
    pub cpui: usize,

    #[arg(long, help = "number of runtime")]
    pub nums: usize,
}

impl RuntimeParam {
    pub fn check(&self) -> CoralRes<()> {
        let limit = num_cpus::get();
        match self.cpui + self.nums {
            x if x > limit => Err(Error::InvalidCpuNum),
            x if x == 0 => Err(Error::InvalidCpuNum),
            _ => Ok(()),
        }
    }
}
