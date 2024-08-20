use crate::error::CoralRes;

mod format;
pub mod logger;
pub use logs_proto::Record;

#[cfg(debug_assertions)]
pub(super) mod logs_proto;
#[cfg(not(debug_assertions))]
pub(super) mod logs_proto {
    include!(concat!(".", "/logs_proto.rs"));
}

pub fn set_logger<C>(coral: logger::Logger<C>) -> CoralRes<()>
where C: logger::Convert + Default + Send + Sync + 'static {
    log::set_boxed_logger(Box::new(coral))?;
    Ok(())
}

pub fn set_proto_logger(f: std::fs::File, level: log::Level) -> CoralRes<()> {
    let global_log = logger::Logger::<logs_proto::Record>::new(level, None, f)?;
    log::set_boxed_logger(Box::new(global_log))?;
    log::set_max_level(level.to_level_filter());
    Ok(())
}

pub fn set_stdout_logger() -> CoralRes<()> {
    let global_log =
        logger::Logger::<logger::Stdout>::new(log::Level::Debug, None, std::io::stdout())?;
    log::set_boxed_logger(Box::new(global_log))?;
    log::set_max_level(log::LevelFilter::Debug);
    Ok(())
}
