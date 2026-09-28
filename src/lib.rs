#![doc = include_str!("../README.md")]

#[cfg(all(
    not(feature = "serial"),
    not(feature = "parallel"),
    not(feature = "async")
))]
compile_error!("At least one of the features 'serial', 'parallel', or 'async' must be enabled");

mod core;

pub mod inode_mapping;
/// Compatibility path for the former mapper module.
#[deprecated(since = "0.7.0", note = "use `easy_fuser::inode_mapping` instead")]
pub mod inode_mapper {
    pub use crate::inode_mapping::*;
}
pub mod session;
pub mod types;
pub mod unix_fs;

pub mod fuse_async;
pub mod fuse_parallel;
pub mod fuse_presets;
pub mod fuse_serial;

pub use easy_fuser_macro::{delegate_fs, delegate_fs_async, delegate_fs_sync_to_async};
pub use session::{FusePruner, FuseSession};
