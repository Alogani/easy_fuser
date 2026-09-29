//! Mirror an existing folder through a FUSE filesystem.
//!
//! `MirrorFs` reads and writes files in the source folder. `MirrorFsReadOnly`
//! provides the read operations only. Both use paths relative to the mount root.
//! Neither type implements `FuseHandler` by itself: put it in your own handler
//! and delegate the methods listed below.
//!
//! # Methods provided
//!
//! Both types provide `access`, `getattr`, `getxattr`, `listxattr`, `lookup`,
//! `open`, `readdir`, `readlink`, `statfs`, `flush`, `fsync`, `lseek`, `read`,
//! and `release`.
//!
//! `MirrorFs` also provides `copy_file_range`, `fallocate`, `write`, `create`,
//! `mkdir`, `mknod`, `removexattr`, `rename`, `rmdir`, `setattr`, `setxattr`,
//! `symlink`, and `unlink`.
//!
//! `MirrorFsReadOnly` does not provide write operations. To make the entire
//! mount read-only, use the FUSE read-only mount option as well.
//!
//! # Methods to provide or delegate elsewhere
//!
//! Neither type provides `bmap`, `forget`, `fsyncdir`, `getlk`, `ioctl`, `link`,
//! `opendir`, `readdirplus`, `releasedir`, or `setlk`. Use
//! [`StatelessHandler`](crate::fuse_presets::StatelessHandler) for the
//! simple directory methods when they suit your filesystem, and
//! [`UnimplementedFuseHandler`](crate::fuse_presets::UnimplementedFuseHandler)
//! for methods your filesystem does not support. Implement any behavior you need.
//!
//! Keep the mount point outside the source folder. Otherwise the filesystem
//! could try to read its own mounted contents recursively.

use std::path::Path;

use super::fd_handler_helper::*;
use crate::types::*;
use crate::unix_fs;

macro_rules! mirror_fs_readonly_methods {
    () => {
        pub fn access(&self, _req: &RequestInfo, file_id: std::path:: PathBuf, mask: AccessFlags) -> FuseResult<()> {
            let file_path = self.source_path.join(file_id);
            unix_fs::access(&file_path, mask)
        }

        pub fn getattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            _file_handle: Option<BorrowedFileHandle>,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(file_id);
            unix_fs::lookup(&file_path)
        }

        pub fn getxattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
            size: u32,
        ) -> FuseResult<Vec<u8>> {
            let file_path = self.source_path.join(file_id);
            unix_fs::getxattr(&file_path, name, size)
        }

        pub fn listxattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            size: u32,
        ) -> FuseResult<Vec<u8>> {
            let file_path = self.source_path.join(file_id);
            unix_fs::listxattr(&file_path, size)
        }

        pub fn lookup(
            &self,
            _req: &RequestInfo,
            parent_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::lookup(&file_path)
        }

        pub fn open(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            flags: OpenFlags,
        ) -> FuseResult<(OwnedFileHandle, FopenFlags)> {
            let file_path = self.source_path.join(file_id);
            let fd = unix_fs::open(file_path.as_ref(), flags)?;
            // Open by definition returns positive Fd or error
            let file_handle = OwnedFileHandle::from_owned_fd(fd).unwrap();
            Ok((file_handle, FopenFlags::empty()))
        }

        pub fn readdir(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            _file_handle: BorrowedFileHandle,
        ) -> FuseResult<Vec<(std::ffi::OsString, FileKind)>> {
            let folder_path = self.source_path.join(file_id);
            let children = unix_fs::readdir(folder_path.as_ref())?;
            let mut result = Vec::new();
            result.push((std::ffi::OsString::from("."), FileKind::Directory));
            result.push((std::ffi::OsString::from(".."), FileKind::Directory));
            for (child_name, child_kind) in children {
                result.push((child_name, child_kind));
            }
            Ok(result)
        }

        pub fn readlink(&self, _req: &RequestInfo, file_id: std::path:: PathBuf) -> FuseResult<Vec<u8>> {
            let file_path = self.source_path.join(file_id);
            unix_fs::readlink(&file_path)
        }

        pub fn statfs(&self, _req: &RequestInfo, file_id: std::path:: PathBuf) -> FuseResult<StatFs> {
            let file_path = self.source_path.join(file_id);
            unix_fs::statfs(&file_path)
        }
    };
}

macro_rules! mirror_fs_readwrite_methods {
    () => {
        pub fn create(
            &self,
            _req: &RequestInfo,
            parent_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
            mode: u32,
            umask: u32,
            flags: OpenFlags,
        ) -> FuseResult<(OwnedFileHandle, FileAttribute, FopenFlags)> {
            let file_path = self.source_path.join(parent_id).join(name);
            let (fd, file_attr) = unix_fs::create(&file_path, mode, umask, flags)?;
            // Open by definition returns positive Fd or error
            let file_handle = OwnedFileHandle::from_owned_fd(fd).unwrap();
            Ok((file_handle, file_attr, FopenFlags::empty()))
        }

        pub fn mkdir(
            &self,
            _req: &RequestInfo,
            parent_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
            mode: u32,
            umask: u32,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::mkdir(&file_path, mode, umask)
        }

        pub fn mknod(
            &self,
            _req: &RequestInfo,
            parent_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
            mode: u32,
            umask: u32,
            rdev: DeviceType,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::mknod(&file_path, mode, umask, rdev)
        }

        pub fn removexattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
        ) -> FuseResult<()> {
            let file_path = self.source_path.join(file_id);
            unix_fs::removexattr(&file_path, name)
        }

        pub fn rename(
            &self,
            _req: &RequestInfo,
            parent_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
            newparent: std::path:: PathBuf,
            newname: &std::ffi::OsStr,
            flags: RenameFlags,
        ) -> FuseResult<()> {
            let oldpath = self.source_path.join(parent_id).join(name);
            let newpath = self.source_path.join(newparent).join(newname);
            unix_fs::rename(&oldpath, &newpath, flags)
        }

        pub fn rmdir(&self, _req: &RequestInfo, parent_id: std::path:: PathBuf, name: &std::ffi::OsStr) -> FuseResult<()> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::rmdir(&file_path)
        }

        pub fn setattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            attrs: SetAttrRequest,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(file_id);
            unix_fs::setattr(&file_path, attrs)
        }

        pub fn setxattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
            value: Vec<u8>,
            flags: SetXAttrFlags,
            position: u32,
        ) -> FuseResult<()> {
            let file_path = self.source_path.join(file_id);
            unix_fs::setxattr(&file_path, name, &value, flags, position)
        }

        pub fn symlink(
            &self,
            _req: &RequestInfo,
            parent_id: std::path:: PathBuf,
            link_name: &std::ffi::OsStr,
            target: &std::path::Path,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(link_name);
            unix_fs::symlink(&file_path, target)
        }

        pub fn unlink(&self, _req: &RequestInfo, parent_id: std::path:: PathBuf, name: &std::ffi::OsStr) -> FuseResult<()> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::unlink(&file_path)
        }
    };
}

pub trait MirrorFsTrait {
    fn new(source_path: std::path:: PathBuf) -> Self;

    fn source_dir(&self) -> &Path;
}

/// Mirrors a source directory and provides read and write file operations.
///
/// Delegate the methods listed in the module documentation to this preset.
/// Implement other operations in your own handler or delegate them to another
/// preset. The source directory must be outside the mount point.
pub struct MirrorFs {
    source_path: std::path:: PathBuf,
}

impl MirrorFsTrait for MirrorFs {
    fn new(source_path: std::path:: PathBuf) -> Self {
        Self { source_path }
    }

    fn source_dir(&self) -> &Path {
        self.source_path.as_path()
    }
}

impl MirrorFs {
    mirror_fs_readonly_methods!();
    mirror_fs_readwrite_methods!();
    fd_handler_readonly_methods!(std::path::PathBuf);
    fd_handler_readwrite_methods!(std::path::PathBuf);
}

/// Read-only mirror of a source directory.
///
/// Provides the read methods listed in the module documentation. It does not
/// reject changes made through other methods in your handler; configure a
/// read-only mount when the whole filesystem must be read-only.
pub struct MirrorFsReadOnly {
    source_path: std::path:: PathBuf,
}

impl MirrorFsTrait for MirrorFsReadOnly {
    fn new(source_path: std::path:: PathBuf) -> Self {
        Self { source_path }
    }

    fn source_dir(&self) -> &Path {
        self.source_path.as_path()
    }
}

impl MirrorFsReadOnly {
    mirror_fs_readonly_methods!();
    fd_handler_readonly_methods!(std::path::PathBuf);
}
