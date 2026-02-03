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
pub mod inode_multi_mapper;
pub mod templates;
pub mod types;
pub mod unix_fs;

pub use fuse_handler::FuseHandler;
use fuser::BackgroundSession;

pub mod prelude {
    //! Re-exports the necessary types and functions from the `easy_fuser` crate.
    pub use super::fuse_handler::FuseHandler;
    pub use super::types::*;
    pub use super::{mount, spawn_mount};
    pub use fuser::{BackgroundSession, MountOption, Session, SessionUnmounter, Config, SessionACL};
}

// Implentation of the high-level functions
use std::io;
use std::path::Path;

use core::FuseDriver;
use fuser::{mount2, spawn_mount2};
use prelude::*;

#[doc = include_str!("../docs/mount.md")]
#[cfg(not(feature = "serial"))]
pub fn mount<T, FS, P>(filesystem: FS, mountpoint: P, config: &Config) -> io::Result<()>
where
    T: FileIdType,
    FS: FuseHandler<T>,
    P: AsRef<Path>,
{
    let mut modified_config = config.clone();
    let n_threads = modified_config.n_threads.unwrap_or(1);
    modified_config.n_threads = Some(n_threads);
    let driver = FuseDriver::new(filesystem, n_threads);
    mount2(driver, mountpoint, &modified_config)
}

#[doc = include_str!("../docs/mount.md")]
#[cfg(feature = "serial")]
pub fn mount<T, FS, P>(filesystem: FS, mountpoint: P, config: &Config) -> io::Result<()>
where
    T: FileIdType,
    FS: FuseHandler<T>,
    P: AsRef<Path>,
{
    // num_thread argument will be forced to 1 due to feature serial
    let mut modified_config = config.clone();
    modified_config.n_threads = Some(1);
    let driver = FuseDriver::new(filesystem, 1);
    mount2(driver, mountpoint, &modified_config)
}

#[doc = include_str!("../docs/spawn_mount.md")]
#[cfg(not(feature = "serial"))]
pub fn spawn_mount<T, FS, P>(
    filesystem: FS,
    mountpoint: P,
    config: &Config,
) -> io::Result<BackgroundSession>
where
    T: FileIdType,
    FS: FuseHandler<T> + Send,
    P: AsRef<Path>,
{
    let mut modified_config = config.clone();
    let n_threads = modified_config.n_threads.unwrap_or(1);
    modified_config.n_threads = Some(n_threads);
    let driver = FuseDriver::new(filesystem, n_threads);
    spawn_mount2(driver, mountpoint, &modified_config)
}

#[doc = include_str!("../docs/spawn_mount.md")]
#[cfg(feature = "serial")]
pub fn spawn_mount<T, FS, P>(
    filesystem: FS,
    mountpoint: P,
    config: &Config,
) -> io::Result<BackgroundSession>
where
    T: FileIdType,
    FS: FuseHandler<T> + Send,
    P: AsRef<Path>,
{
    // num_thread argument will be forced to 1 due to feature serial
    let mut modified_config = config.clone();
    modified_config.n_threads = Some(1);
    let driver = FuseDriver::new(filesystem, 1);
    spawn_mount2(driver, mountpoint, &modified_config)
}
