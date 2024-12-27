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

pub mod templates;

pub mod posix_fs;
pub mod types;

pub mod prelude {
    pub use super::fuse_handler::FuseHandler;
    pub use super::types::*;
    pub use super::{mount, spawn_mount};

    pub use fuser::{BackgroundSession, MountOption, Session, SessionUnmounter};
}

// Implentation of the high-level functions
use std::io;
use std::path::Path;

use core::FuseDriver;
use fuser::{mount2, spawn_mount2};
use prelude::{BackgroundSession, FileIdType, FuseHandler, MountOption};

/// Mounts a FUSE filesystem at the specified mountpoint.
///
/// # Parameters
///
/// * `filesystem`: The filesystem implementation.
/// * `mountpoint`: The path where the filesystem should be mounted.
/// * `options`: Mount options for the filesystem.
/// * `num_threads`: Number of threads for handling filesystem operations.
///
/// # Type Parameters
///
/// * `T`: Implements `FileIdType` for file identifier conversion.
/// * `FS`: Implements `FuseHandler<T>` for filesystem operations.
///
/// # Returns
///
/// `io::Result<()>` indicating success or failure of the mount operation.
///
/// # Panics
///
/// When the `serial` feature is enabled, this function will panic at compile-time if `num_threads` is greater than 1.
pub fn mount<T, FS>(
    filesystem: FS,
    mountpoint: &Path,
    options: &[MountOption],
    num_threads: usize,
) -> io::Result<()>
where
    T: FileIdType,
    FS: FuseHandler<T>,
{
    #[cfg(feature = "serial")]
    if num_threads > 1 {
        panic!("num_threads cannot be superior to 1 when feature serial is enabled");
    }
    let driver = FuseDriver::new(filesystem, num_threads);
    mount2(driver, mountpoint, options)
}

/// Spawns a FUSE filesystem in the background at the specified mountpoint.
///
/// This function mounts a FUSE filesystem and returns a `BackgroundSession` that can be used
/// to manage the mounted filesystem.
///
/// # Parameters
///
/// * `filesystem`: The filesystem implementation that handles FUSE operations.
/// * `mountpoint`: The path where the filesystem should be mounted.
/// * `options`: A slice of mount options for configuring the filesystem mount.
/// * `num_threads`: Number of threads for handling filesystem operations concurrently.
///
/// # Type Parameters
///
/// * `T`: Implements `FileIdType` for file identifier conversion.
/// * `FS`: Implements `FuseHandler<T>` for filesystem operations.
///
/// # Returns
///
/// Returns `io::Result<BackgroundSession>`, which is:
/// * `Ok(BackgroundSession)` on successful mount, providing a handle to manage the mounted filesystem.
/// * `Err(io::Error)` if the mount operation fails.
///
/// # Panics
///
/// When the `serial` feature is enabled, this function will panic at compile-time if `num_threads` is greater than 1.
pub fn spawn_mount<T, FS>(
    filesystem: FS,
    mountpoint: &Path,
    options: &[MountOption],
    num_threads: usize,
) -> io::Result<BackgroundSession>
where
    T: FileIdType,
    FS: FuseHandler<T>,
{
    #[cfg(feature = "serial")]
    if num_threads > 1 {
        panic!("num_threads cannot be superior to 1 when feature serial is enabled");
    }
    let driver = FuseDriver::new(filesystem, num_threads);
    spawn_mount2(driver, mountpoint, options)
}
