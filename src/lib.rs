mod core;
mod fuse_handler;
mod types;

pub mod posix_fs;
pub mod templates;

pub mod prelude {
    pub use super::core::new_serial_driver;
    pub use super::fuse_handler::FuseHandler;
    pub use super::types::*;
    pub use fuser::{mount2 as mount, spawn_mount2 as spawn_mount, MountOption};
}
