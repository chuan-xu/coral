mod error;
use crate::error::CoralRes;
use coral_conf::EnvAssignToml;
use coral_macro::EnvAssign;
use core_affinity::CoreId;
pub use error::Error;
use serde::Deserialize;
use std::{future::Future, sync::atomic};
pub use tokio;

#[derive(Deserialize, Debug, EnvAssign)]
pub struct RuntimeConf {
    cpui: usize,
    nums: usize,
}

impl RuntimeConf {
    pub fn check(&self) -> CoralRes<()> {
        let limit = num_cpus::get();
        match self.cpui + self.nums {
            x if x > limit => Err(Error::InvalidCpuNum),
            x if x == 0 => Err(Error::InvalidCpuNum),
            _ => Ok(()),
        }
    }

    /// `start` - 选定的cpu核数起始索引
    /// `nums` - 异步运行时的线程数，不包含当前线程
    /// `th_name_pre` - 线程名前缀
    pub fn runtime(&self, th_name_pre: &'static str) -> Result<tokio::runtime::Runtime, Error> {
        let _cores = cpu_cores(self.cpui, self.nums + 1)?;
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(self.nums)
            .enable_all()
            .thread_name_fn(move || {
                static ATOMIC_ID: atomic::AtomicUsize = atomic::AtomicUsize::new(0);
                let id = ATOMIC_ID.fetch_add(1, atomic::Ordering::SeqCst);
                format!("{}-{}", th_name_pre, id)
            })
            .on_thread_start(move || {
                #[cfg(debug_assertions)]
                println!("[+] create runtime thread in tokio");
                // BUG
                // if let Ok(index) = get_thread_index() {
                //     if !core_affinity::set_for_current(cores[index].clone()) {
                //         log::error!("failed to core affinity");
                //     }
                // } else {
                //     log::error!("failed to get thread index on thread start");
                // }
            })
            .on_thread_stop(|| {
                #[cfg(debug_assertions)]
                println!("[+] stop runtime thread in tokio");
            })
            .build()?;
        Ok(rt)
    }
}

/// 从`th_name`中提取线程编号
#[allow(unused)]
fn get_thread_index() -> Result<usize, Error> {
    let current = std::thread::current();
    let name = current.name().ok_or(Error::NoneThreadName)?;
    let ix = name.rfind("-").ok_or(Error::NoneThreadIndex)?;
    Ok(usize::from_str_radix(&name[ix + 1..], 10)?)
}

// struct Core {
//     id: CoreId,
//     count: atomic::AtomicUsize
// }

/// 获取从`start`开始的共`nums`个的cores
fn cpu_cores(start: usize, nums: usize) -> Result<Vec<core_affinity::CoreId>, Error> {
    let cores = core_affinity::get_core_ids().ok_or(Error::NoneCoreIds)?;
    if start + nums > cores.len() {
        return Err(Error::NoneCoreIds);
    }
    // for i in start..nums {
    //     if let Some(c) = cores.get(i)
    // }
    Ok((0..nums).map(|i| cores[start + i]).collect())
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
    use super::*;

    #[test]
    fn get_cpu_cores() {
        let cpus = num_cpus::get();
        assert!(cpu_cores(1, cpus).is_err());
    }

    #[test]
    fn split_thname() {
        let thname = "coral-proxy-1";
        let idx = thname.rfind("-").unwrap();
        let n = usize::from_str_radix(&thname[idx + 1..], 10).unwrap();
        assert_eq!(n, 1);
    }
}
