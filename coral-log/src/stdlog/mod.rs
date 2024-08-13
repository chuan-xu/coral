pub mod format;
pub mod io;

#[cfg(debug_assertions)]
pub(super) mod record_proto;
#[cfg(not(debug_assertions))]
pub(super) mod record_proto {
    include!(concat!(".", "/record_proto.rs"));
}
