#![doc = include_str!("../README.md")]

#[cfg(feature = "async")]
compile_error!("Feature 'async' is not yet implemented.");

#[cfg(all(
    not(feature = "serial"),
    not(feature = "parallel"),
    not(feature = "async")
))]
compile_error!("At least one of the features 'serial', 'parallel', or 'async' must be enabled");

pub mod fuse_async;
mod fuse_common;
pub mod fuse_parallel;
pub mod fuse_serial;

pub mod inode_mapper;
pub mod types;
pub mod unix_fs;

pub mod prelude {
    //! Re-exports the necessary types and functions from the `easy_fuser` crate.
    pub use super::types::*;

    pub use fuser::{BackgroundSession, MountOption, Session, SessionUnmounter};
}
