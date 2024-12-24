//! # easy_fuser
//!
//! `easy_fuser` is a high-level, ergonomic wrapper around the `fuser` crate, designed to simplify
//! the process of implementing FUSE (Filesystem in Userspace) filesystems in Rust. It abstracts away
//! many of the complexities, offering a more intuitive and Rust-idiomatic approach to filesystem development.
//!
//! ## Key Features
//!
//! - **Simplified API**: Provides a higher-level interface compared to `fuser`, reducing boilerplate
//!   and making filesystem implementation more straightforward.
//!
//! - **Flexible Concurrency Models**: Offers three distinct concurrency models to suit different
//!   use cases and performance requirements.
//!
//! - **Flexible File Identification**: Supports both path-based and inode-based operations,
//!   allowing you to choose between `Inode` and `PathBuf` as your file identifier type. This
//!   offers flexibility in how you represent and manage file identities, suitable for different
//!   filesystem structures and performance requirements.
//!
//! - **Error Handling**: Provides a structured error handling system, facilitating the management
//!   of filesystem-specific errors.
//!
//! - **Templates and Examples**: Includes pre-built templates and a comprehensive examples folder
//!   to help you get started quickly and understand various implementation patterns.
//!
//! ## File Identification Flexibility
//!
//! `easy_fuser` supports two main approaches for file identification:
//!
//! 1. **Path-based Operations**: Work with file paths directly, which can be more intuitive for
//!    certain use cases.
//! 2. **Inode-based Operations**: Use inode numbers for more efficient control, especially useful
//!    for complex filesystem structures or when performance is critical.
//!
//! You can choose the approach that best fits your filesystem's needs and switch between them
//! as necessary.
//!
//! ## Usage
//!
//! To use `easy_fuser`, follow these steps:
//!
//! 1. Implement the `FuseHandler` trait for your filesystem structure.
//! 2. (Optional) Utilize provided templates to jumpstart your implementation.
//! 3. Choose an appropriate concurrency model by enabling the corresponding feature.
//! 4. Use the `mount` or `spawn_mount` functions to start your filesystem.
//!
//! Here's a basic example:
//!
//! ```rust,no_run
//! use easy_fuser::prelude::*;
//! use easy_fuser::templates::DefaultFuseHandler;
//! use std::path::{Path, PathBuf};
//!
//! struct MyFS {
//!     inner: DefaultFuseHandler,
//! }
//!
//! impl FuseHandler<PathBuf> for MyFS {
//!     fn get_inner(&self) -> &dyn FuseHandler<PathBuf> {
//!         &self.inner
//!     }
//! }
//!
//! fn main() -> std::io::Result<()> {
//!     let fs = MyFS { inner: DefaultFuseHandler::new() };
//!     easy_fuser::mount(fs, Path::new("/mnt/myfs"), &[], 1)
//! }
//! ```
//!
//! ## Templates
//!
//! `easy_fuser` provides a set of templates to help you get started quickly:
//!
//! - **DefaultFuseHandler**: A backbone implementation that acts as a NullFs, implementing every
//!   operation. It can also be used as a PanicFs for debugging purposes.
//! - **FdHandlerHelper**: Provides boilerplate for operations on open files (ReadOnly and ReadWrite variants available)
//! - **MirrorFs**: A passthrough filesystem that can be leveraged for creating more complex filesystems.
//!
//! These templates serve as starting points or building blocks for your custom filesystem implementations.
//!
//! ## Examples
//!
//! The `examples` folder in the repository is currently under construction. It is expected to include
//! various implementations demonstrating different aspects of filesystem creation, including:
//!
//! - ZipFs: A filesystem for browsing and accessing zip archives.
//! - FtpFs: A filesystem that provides access to FTP servers.
//! - SqlFs: A filesystem that represents SQL database contents.
//!
//! These examples, once completed, will serve as valuable references when building your own filesystem.
//!
//! ## Feature Flags
//!
//! This crate provides three mutually exclusive feature flags for different concurrency models:
//!
//! - `serial`: Enables single-threaded operation. Use this for simplicity and when concurrent
//!   access is not required. When this feature is enabled, `num_threads` must be set to 1.
//!
//! - `parallel`: Enables multi-threaded operation using a thread pool. This is suitable for
//!   scenarios where you want to handle multiple filesystem operations concurrently on separate
//!   threads. It can improve performance on multi-core systems.
//!
//! - `async`: Enables asynchronous operation. This is ideal for high-concurrency scenarios and
//!   when you want to integrate the filesystem with asynchronous Rust code. It allows for
//!   efficient handling of many concurrent operations without the overhead of threads.
//!
//! You must enable exactly one of these features when using this crate. The choice depends on
//! your specific use case and performance requirements.
//!
//! Example usage in Cargo.toml:
//! ```toml
//! [dependencies]
//! easy_fuser = { version = "0.1.0", features = ["parallel"] }
//! ```
//!
//! By leveraging `easy_fuser`, you can focus more on your filesystem's logic and less on the
//! intricacies of FUSE implementation, making it easier to create robust, efficient, and
//! maintainable filesystem solutions in Rust.

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

    pub use fuser::{BackgroundSession, MountOption};
}

// Implentation of the high-level functions
use std::io;
use std::path::Path;

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
    let id_resolver = T::get_converter();
    let driver = core::FuseDriver::new(filesystem, id_resolver, num_threads);
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
    let id_resolver = T::get_converter();
    let driver = core::FuseDriver::new(filesystem, id_resolver, num_threads);
    spawn_mount2(driver, mountpoint, options)
}
