use std::io;
use std::path::Path;

use super::{FuseDriver, FuseHandler};
use crate::types::*;
use fuser::{mount2, spawn_mount2};
pub use fuser::{BackgroundSession, MountOption, Session, SessionUnmounter};

#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/mount.md"))]
pub fn mount<T, FS, P>(filesystem: FS, mountpoint: P, options: &[MountOption]) -> io::Result<()>
where
    T: FileIdType,
    FS: FuseHandler<T>,
    P: AsRef<Path>,
{
    let driver = FuseDriver::new(filesystem, 1);
    mount2(driver, mountpoint, options)
}

#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/spawn_mount.md"))]

pub fn spawn_mount<T, FS, P>(
    filesystem: FS,
    mountpoint: P,
    options: &[MountOption],
) -> io::Result<BackgroundSession>
where
    T: FileIdType,
    FS: FuseHandler<T> + Send,
    P: AsRef<Path>,
{
    let driver = FuseDriver::new(filesystem, 1);
    spawn_mount2(driver, mountpoint, options)
}
