#![doc = include_str!("../README.md")]

#[cfg(feature = "async")]
compile_error!("Feature 'async' is not yet implemented.");

#[cfg(all(
    not(feature = "serial"),
    not(feature = "parallel"),
    not(feature = "async")
))]
compile_error!("At least one of the features 'serial', 'parallel', or 'async' must be enabled");

#[cfg(all(feature = "serial", any(feature = "parallel", feature = "async")))]
compile_error!("Feature 'serial' cannot be used with feature parallel or async");

#[cfg(all(feature = "parallel", any(feature = "serial", feature = "async")))]
compile_error!("Feature 'parallel' cannot be used with feature serial or async");

#[cfg(all(feature = "async", any(feature = "serial", feature = "parallel")))]
compile_error!("Feature 'async' cannot be used with feature serial or parallel");

mod core;
mod fuse_handler;

pub mod inode_mapper;
pub mod session;
pub mod inode_multi_mapper;
pub mod templates;
pub mod types;
pub mod unix_fs;

pub use fuse_handler::FuseHandler;
pub use session::{FusePruner, FuseSession};

pub mod prelude {
    //! Re-exports the necessary types and functions from the `easy_fuser` crate.
    pub use super::fuse_handler::FuseHandler;
    pub use super::session::{FusePruner, FuseSession};
    pub use super::types::*;
    pub use super::{mount, spawn_mount};

    pub use fuser::{BackgroundSession, MountOption, Session, SessionUnmounter};
}


#[cfg(feature = "serial")]
include!(concat!(env!("OUT_DIR"), "/serial/mouting.rs"));
#[cfg(not(feature = "serial"))]
include!(concat!(env!("OUT_DIR"), "/parallel/mouting.rs"));