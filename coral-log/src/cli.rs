use clap::Parser;
use opentelemetry::KeyValue;

use crate::{
    error::{CoralRes, Error},
    logs,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(next_line_help = true)]
pub struct Param {
    #[arg(long, help = "directory for storing logs")]
    pub dir: Option<String>,

    #[arg(long, help = "Log file name prefix")]
    pub prefix: Option<String>,

    #[arg(long, help = "Log file splitting period")]
    pub rotation: Option<String>,

    #[arg(long, help = "telemetry collector address")]
    pub otel_endpoint: Option<String>,

    #[arg(long, help = "telemetry resource key value")]
    pub otel_kvs: Vec<String>,
}

impl Param {
    pub fn check(&self) -> CoralRes<()> {
        if let Some(dir) = self.dir.as_ref() {
            if !std::fs::metadata(dir)?.is_dir() {
                return Err(Error::InvalidLogDir);
            }
        }
        self.set_log()?;
        self.set_traces();
        Ok(())
    }

    fn set_log(&self) -> CoralRes<()> {
        if self.dir.is_some() && self.prefix.is_some() {
            let path = std::path::Path::new(self.dir.as_ref().unwrap());
            let file = path.join(self.prefix.as_ref().unwrap());
            let fd = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(file)?;
            logs::set_proto_logger(fd, log::Level::Info)?;
        } else {
            logs::set_stdout_logger()?;
        }
        Ok(())
    }

    fn set_traces(&self) {
        if let Some(endpoint) = self.otel_endpoint.as_ref() {
            super::traces::otel_trace(endpoint, self.get_otel_kvs())
        }
    }

    fn get_otel_kvs(&self) -> Vec<KeyValue> {
        let mut kvs = Vec::new();
        for kv in self.otel_kvs.iter() {
            if let Some((k, v)) = kv.split_once("=") {
                kvs.push(KeyValue::new(k.to_owned(), v.to_owned()));
            }
        }
        kvs
    }
}
