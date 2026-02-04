#![doc = include_str!("../README.md")]

#[cfg(feature = "async")]
compile_error!("Feature 'async' is not yet implemented.");

#[cfg(all(
    not(feature = "serial"),
    not(feature = "parallel"),
    not(feature = "async")
))]
compile_error!("At least one of the features 'serial', 'parallel', or 'async' must be enabled");

mod core;

pub mod inode_mapper;
pub mod inode_multi_mapper;
pub mod session;
// TODO :pub mod templates;
pub mod types;
pub mod unix_fs;

pub mod fuse_async;
pub mod fuse_parallel;
pub mod fuse_serial;


pub use session::{FusePruner, FuseSession};
