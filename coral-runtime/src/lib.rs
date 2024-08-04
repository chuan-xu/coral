mod error;

pub use tokio;

pub use error::Error;
use std::sync::atomic;

/// `start` - 选定的cpu核数起始索引
/// `nums` - 异步运行时的线程数，不包含当前线程
/// `th_name_pre` - 线程名前缀
pub fn runtime<F>(
    start: usize,
    nums: usize,
    th_name_pre: &'static str,
    before_f: F,
) -> Result<tokio::runtime::Runtime, Error>
where
    F: Fn() + Send + Sync + 'static,
{
    let cores = cpu_cores(start, nums)?;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(nums)
        .enable_all()
        .thread_name_fn(move || {
            static ATOMIC_ID: atomic::AtomicUsize = atomic::AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, atomic::Ordering::SeqCst);
            format!("{}-{}", th_name_pre, id)
        })
        .on_thread_start(move || {
            if let Ok(index) = get_thread_index() {
                if !core_affinity::set_for_current(cores[index].clone()) {
                    eprintln!("failed to core affinity");
                }
            } else {
                eprintln!("failed to get thread index on thread start");
            }
            before_f();
        })
        .build()?;
    Ok(rt)
}

/// 从`th_name`中提取线程编号
fn get_thread_index() -> Result<usize, Error> {
    let current = std::thread::current();
    let name = current.name().ok_or(Error::NoneThreadName)?;
    name.rfind("-").ok_or(Error::NoneThreadIndex)
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

    #[test]
    fn split_thname() {
        let thname = "coral-proxy-1";
        let idx = thname.rfind("-").unwrap();
        println!("{:?}", &thname[idx..]);
    }
}
