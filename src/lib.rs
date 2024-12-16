mod fuse_handler;
pub use fuse_handler::FuseHandler;

pub mod posix_fs;
pub mod types;
mod core;
pub use core::new_serial_driver;

pub mod templates;

pub use fuser::{mount2 as mount, spawn_mount2 as spawn_mount, MountOption};
