pub mod error;
pub mod logs;
pub mod metrics;
pub mod traces;

use opentelemetry::KeyValue;

use crate::error::CoralRes;
use crate::error::Error;
use coral_conf::EnvAssignToml;
use coral_macro::EnvAssign;
use serde::Deserialize;

#[derive(Deserialize, EnvAssign, Debug, Clone)]
pub struct LogConf {
    dir: Option<String>,
    prefix: Option<String>,
    rotation: Option<String>,
    otel_endpoint: Option<String>,
    otel_kvs: Option<Vec<String>>,
}

impl LogConf {
    pub fn check(&self) -> CoralRes<()> {
        if let Some(dir) = self.dir.as_ref() {
            if !std::fs::metadata(dir)?.is_dir() {
                return Err(Error::InvalidLogDir);
            }
        }
        self.set_log()?;
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

    pub fn set_traces(&self) -> Option<impl FnOnce()> {
        if let Some(endpoint) = self.otel_endpoint.as_ref() {
            let otel_kvs = self.get_otel_kvs();
            let endpoint = endpoint.to_owned();
            let t = Some(move || traces::otel_trace(endpoint, otel_kvs));
            t
        } else {
            None
        }
    }

    fn get_otel_kvs(&self) -> Vec<KeyValue> {
        let mut kvs = Vec::new();
        if let Some(otel_kvs) = self.otel_kvs.as_ref() {
            for kv in otel_kvs.iter() {
                if let Some((k, v)) = kv.split_once("=") {
                    kvs.push(KeyValue::new(k.to_owned(), v.to_owned()));
                }
            }
        }
        kvs
    }
}
