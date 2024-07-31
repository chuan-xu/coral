mod error;

pub use tokio;

use error::Error;
use std::sync::atomic;

/// `start` - 选定的cpu核数起始索引
/// `nums` - 异步运行时的线程数，不包含当前线程
/// `th_name_pre` - 线程名前缀
pub fn runtime(start: usize, nums: usize, th_name_pre: &'static str) -> Result<(), Error> {
    let cores = cpu_cores(start, nums)?;
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(nums)
        .enable_all()
        .thread_name_fn(move || {
            static ATOMIC_ID: atomic::AtomicUsize = atomic::AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, atomic::Ordering::SeqCst);
            format!("{}-{}", th_name_pre, id)
        })
        .on_thread_start(|| {
            if let Some(thnm) = std::thread::current().name() {
                // thnm.sp
            } else {
            }
        });
    Ok(())
}

/// 获取从`start`开始的共`nums`个的cores
fn cpu_cores(start: usize, nums: usize) -> Result<Vec<core_affinity::CoreId>, Error> {
    let cores = core_affinity::get_core_ids().ok_or(Error::NoneCoreIds)?;
    if start + nums > cores.len() {
        return Err(Error::NoneCoreIds);
    }
    Ok((0..nums).map(|i| cores[start + i]).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_cpu_cores() {
        let cpus = num_cpus::get();
        assert!(cpu_cores(1, cpus).is_err());
    }
}
