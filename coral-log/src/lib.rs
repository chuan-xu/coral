pub use error::Error;
pub use log;
pub use stdlog::io::Coralog;
pub use stdlog::io::Stdout;
pub use stdlog::record_proto::Record;
pub use stdlog::set_logger;

mod error;
#[cfg(feature = "stdlog")]
mod stdlog;

#[cfg(feature = "tktrace")]
mod tktrace;

#[cfg(test)]
mod tests;
