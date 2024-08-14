use io::Convert;

use crate::error::CoralRes;

pub mod format;
pub mod io;

#[cfg(debug_assertions)]
pub(super) mod record_proto;
#[cfg(not(debug_assertions))]
pub(super) mod record_proto {
    include!(concat!(".", "/record_proto.rs"));
}

pub fn set_logger<C>(coral: io::Coralog<C>) -> CoralRes<()>
where C: Convert + Default + Send + Sync + 'static {
    log::set_boxed_logger(Box::new(coral))?;
    Ok(())
}
