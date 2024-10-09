mod error;
use crate::error::CoralRes;
use coral_conf::EnvAssignToml;
use coral_macro::EnvAssign;
pub use error::Error;
use serde::Deserialize;
use std::{future::Future, sync::atomic};
pub use tokio;

#[derive(Deserialize, Debug, EnvAssign)]
pub struct RuntimeConf {
    cpui: usize,
    nums: usize,
}

static ATOMIC_ID: atomic::AtomicUsize = atomic::AtomicUsize::new(0);

impl RuntimeConf {
    pub fn check(&self) -> CoralRes<()> {
        let limit = num_cpus::get();
        match self.cpui + self.nums {
            x if x > limit => Err(Error::InvalidCpuNum),
            x if x == 0 => Err(Error::InvalidCpuNum),
            _ => Ok(()),
        }
    }

    pub fn runtime(&self, th_name_pre: &'static str) -> Result<tokio::runtime::Runtime, Error> {
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        builder
            .worker_threads(self.nums)
            .enable_all()
            .thread_name_fn(move || {
                let id = ATOMIC_ID.fetch_add(1, atomic::Ordering::SeqCst);
                format!("{}-{}", th_name_pre, id)
            });
        let cores = cpu_cores(self.cpui, self.nums + 1);
        if cores.len() > 0 {
            builder.on_thread_start(move || {
                #[cfg(debug_assertions)]
                println!("[+] create runtime thread in tokio");
                if let Ok(index) = get_thread_index() {
                    if !core_affinity::set_for_current(cores[index % cores.len()].clone()) {
                        log::error!("failed to core affinity");
                    }
                } else {
                    log::error!("failed to get thread index on thread start");
                }
            });
        }
        Ok(builder.build()?)
    }
}

/// get index from thread_name
#[allow(unused)]
fn get_thread_index() -> Result<usize, Error> {
    let current = std::thread::current();
    let name = current.name().ok_or(Error::NoneThreadName)?;
    let ix = name.rfind("-").ok_or(Error::NoneThreadIndex)?;
    Ok(usize::from_str_radix(&name[ix + 1..], 10)?)
}

fn cpu_cores(start: usize, nums: usize) -> Vec<core_affinity::CoreId> {
    let mut cores = Vec::with_capacity(nums);
    if let Some(ids) = core_affinity::get_core_ids() {
        for i in 0..nums {
            if let Some(core) = ids.get(start + i) {
                cores.push(core.clone());
            } else {
                break;
            }
        }
    }
    cores
}

pub fn spawn<Fut>(future: Fut) -> tokio::task::JoinHandle<Fut::Output>
where
    Fut: Future + Send + 'static,
    Fut::Output: Send + 'static,
{
    tokio::spawn(future)
}

#[cfg(test)]
mod tests {

    #[test]
    fn split_thname() {
        let thname = "coral-proxy-1";
        let idx = thname.rfind("-").unwrap();
        let n = usize::from_str_radix(&thname[idx + 1..], 10).unwrap();
        assert_eq!(n, 1);
    }
}
