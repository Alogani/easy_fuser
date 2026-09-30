//! Mirror an existing folder through a FUSE filesystem.
//!
//! `MirrorFs` reads and writes files in the source folder. `MirrorFsReadOnly`
//! provides the read operations only. Both use paths relative to the mount root.
//! Neither type implements `FuseHandler` by itself: put it in your own handler
//! and delegate the methods listed below.
//!
//! # Methods provided
//!
//! Both types provide `access`, `bmap`, `getattr`, `getlk`, `getxattr`, `ioctl`,
//! `listxattr`, `lookup`, `open`, `readdir`, `readdirplus`, `readlink`, `statfs`,
//! `flush`, `fsync`, `lseek`, `read`, `release`, and `setlk`.
//!
//! `MirrorFs` also provides `copy_file_range`, `fallocate`, `write`, `create`,
//! `mkdir`, `mknod`, `removexattr`, `rename`, `rmdir`, `setattr`, `setxattr`,
//! `symlink`, and `unlink`.
//!
//! `MirrorFsReadOnly` does not provide write operations. To make the entire
//! mount read-only, use the FUSE read-only mount option as well.
//!
//! With the `async` feature, `MirrorFsAsync` and `MirrorFsReadOnlyAsync`
//! provide the same operations as async methods for use with
//! `delegate_fs_async!`. On Linux, enabling `io_uring` makes their
//! descriptor-backed `read`, `write`, `flush`, `fsync`, and `fallocate`
//! operations use io_uring. Path, metadata, namespace, and other operations
//! remain synchronous, as do all operations on BSD/macOS.
//! The current implementation creates and drops a ring for each operation and
//! measured roughly 3–10x lower throughput than async syscalls on warm local
//! files. This is an implementation-specific result, not evidence that io_uring
//! itself is inherently slower; keep the feature experimental and opt-in until
//! persistent-ring reuse and concurrent submissions are measured.
//!
//! # Methods to provide or delegate elsewhere
//!
//! Neither type provides `forget`, `fsyncdir`, `link`, `opendir`, or
//! `releasedir`. Use
//! [`StatelessHandler`](crate::fuse_presets::StatelessHandler) for the
//! simple directory methods when they suit your filesystem, and
//! [`UnimplementedFuseHandler`](crate::fuse_presets::UnimplementedFuseHandler)
//! for methods your filesystem does not support. Implement any behavior you need.
//!
//! Keep the mount point outside the source folder. Otherwise the filesystem
//! could try to read its own mounted contents recursively.

use std::os::fd::OwnedFd;
use std::path::Path;

use super::file_descriptor_handler::*;
use crate::types::*;
use crate::unix_fs;

macro_rules! mirror_fs_readonly_methods {
    ($( $asyncness:ident )?) => {
        pub $( $asyncness )? fn access(
            &self,
            _req: &RequestInfo,
            file_id: std::path::PathBuf,
            mask: AccessFlags,
        ) -> FuseResult<()> {
            let file_path = self.source_path.join(file_id);
            unix_fs::access(&file_path, mask)
        }

        pub $( $asyncness )? fn getattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path::PathBuf,
            _file_handle: Option<&mut OwnedFd>,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(file_id);
            unix_fs::lookup(&file_path)
        }

        pub $( $asyncness )? fn getxattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path::PathBuf,
            name: &std::ffi::OsStr,
            size: u32,
        ) -> FuseResult<Vec<u8>> {
            let file_path = self.source_path.join(file_id);
            unix_fs::getxattr(&file_path, name, size)
        }

        pub $( $asyncness )? fn listxattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path::PathBuf,
            size: u32,
        ) -> FuseResult<Vec<u8>> {
            let file_path = self.source_path.join(file_id);
            unix_fs::listxattr(&file_path, size)
        }

        pub $( $asyncness )? fn lookup(
            &self,
            _req: &RequestInfo,
            parent_id: std::path::PathBuf,
            name: &std::ffi::OsStr,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::lookup(&file_path)
        }

        pub $( $asyncness )? fn open(
            &self,
            _req: &RequestInfo,
            file_id: std::path::PathBuf,
            flags: OpenFlags,
        ) -> FuseResult<(OwnedFd, FopenFlags)> {
            let file_path = self.source_path.join(file_id);
            let fd = unix_fs::open(file_path.as_ref(), flags)?;
            Ok((fd, FopenFlags::empty()))
        }

        pub $( $asyncness )? fn readdir<'a>(
            &self,
            _req: &RequestInfo,
            file_id: std::path::PathBuf,
            _file_handle: BorrowedFileHandle<'a>,
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

        pub $( $asyncness )? fn readlink(
            &self,
            _req: &RequestInfo,
            file_id: std::path::PathBuf,
        ) -> FuseResult<Vec<u8>> {
            let file_path = self.source_path.join(file_id);
            unix_fs::readlink(&file_path)
        }

        pub $( $asyncness )? fn statfs(
            &self,
            _req: &RequestInfo,
            file_id: std::path::PathBuf,
        ) -> FuseResult<StatFs> {
            let file_path = self.source_path.join(file_id);
            unix_fs::statfs(&file_path)
        }
    };
}

fn mirror_readdirplus(
    source_path: &Path,
    file_id: std::path::PathBuf,
) -> FuseResult<Vec<(std::ffi::OsString, FileAttribute)>> {
    let folder_path = source_path.join(&file_id);
    let mut entries = vec![
        (std::ffi::OsString::from("."), FileKind::Directory),
        (std::ffi::OsString::from(".."), FileKind::Directory),
    ];
    entries.extend(unix_fs::readdir(&folder_path)?);

    let mut result = Vec::with_capacity(entries.len());
    for (name, _) in entries {
        let entry_path = match name.as_os_str() {
            name if name == std::ffi::OsStr::new(".") => folder_path.clone(),
            name if name == std::ffi::OsStr::new("..") => {
                let parent = file_id.parent().unwrap_or(std::path::Path::new(""));
                source_path.join(parent)
            }
            _ => folder_path.join(&name),
        };
        result.push((name, unix_fs::lookup(&entry_path)?));
    }
    Ok(result)
}

macro_rules! mirror_fs_readwrite_methods {
    ($( $asyncness:ident )?) => {
        pub $( $asyncness )? fn create(
            &self,
            _req: &RequestInfo,
            parent_id: std::path::PathBuf,
            name: &std::ffi::OsStr,
            mode: u32,
            umask: u32,
            flags: OpenFlags,
        ) -> FuseResult<(OwnedFd, FileAttribute, FopenFlags)> {
            let file_path = self.source_path.join(parent_id).join(name);
            let (fd, file_attr) = unix_fs::create(&file_path, mode, umask, flags)?;
            Ok((fd, file_attr, FopenFlags::empty()))
        }

        pub $( $asyncness )? fn mkdir(
            &self,
            _req: &RequestInfo,
            parent_id: std::path::PathBuf,
            name: &std::ffi::OsStr,
            mode: u32,
            umask: u32,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::mkdir(&file_path, mode, umask)
        }

        pub $( $asyncness )? fn mknod(
            &self,
            _req: &RequestInfo,
            parent_id: std::path::PathBuf,
            name: &std::ffi::OsStr,
            mode: u32,
            umask: u32,
            rdev: DeviceType,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::mknod(&file_path, mode, umask, rdev)
        }

        pub $( $asyncness )? fn removexattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path::PathBuf,
            name: &std::ffi::OsStr,
        ) -> FuseResult<()> {
            let file_path = self.source_path.join(file_id);
            unix_fs::removexattr(&file_path, name)
        }

        pub $( $asyncness )? fn rename(
            &self,
            _req: &RequestInfo,
            parent_id: std::path::PathBuf,
            name: &std::ffi::OsStr,
            newparent: std::path::PathBuf,
            newname: &std::ffi::OsStr,
            flags: RenameFlags,
        ) -> FuseResult<()> {
            let oldpath = self.source_path.join(parent_id).join(name);
            let newpath = self.source_path.join(newparent).join(newname);
            unix_fs::rename(&oldpath, &newpath, flags)
        }

        pub $( $asyncness )? fn rmdir(
            &self,
            _req: &RequestInfo,
            parent_id: std::path::PathBuf,
            name: &std::ffi::OsStr,
        ) -> FuseResult<()> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::rmdir(&file_path)
        }

        pub $( $asyncness )? fn setattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path::PathBuf,
            attrs: SetAttrRequest,
            _file_handle: Option<&mut OwnedFd>,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(file_id);
            unix_fs::setattr(&file_path, attrs)
        }

        pub $( $asyncness )? fn setxattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path::PathBuf,
            name: &std::ffi::OsStr,
            value: Vec<u8>,
            flags: SetXAttrFlags,
            position: u32,
        ) -> FuseResult<()> {
            let file_path = self.source_path.join(file_id);
            unix_fs::setxattr(&file_path, name, &value, flags, position)
        }

        pub $( $asyncness )? fn symlink(
            &self,
            _req: &RequestInfo,
            parent_id: std::path::PathBuf,
            link_name: &std::ffi::OsStr,
            target: &std::path::Path,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(link_name);
            unix_fs::symlink(&file_path, target)
        }

        pub $( $asyncness )? fn unlink(
            &self,
            _req: &RequestInfo,
            parent_id: std::path::PathBuf,
            name: &std::ffi::OsStr,
        ) -> FuseResult<()> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::unlink(&file_path)
        }
    };
}

pub trait MirrorFsTrait {
    fn new(source_path: std::path::PathBuf) -> Self;

    fn source_dir(&self) -> &Path;
}

/// Mirrors a source directory and provides read and write file operations.
///
/// Delegate the methods listed in the module documentation to this preset.
/// Implement other operations in your own handler or delegate them to another
/// preset. The source directory must be outside the mount point.
pub struct MirrorFs {
    source_path: std::path::PathBuf,
}

impl MirrorFsTrait for MirrorFs {
    fn new(source_path: std::path::PathBuf) -> Self {
        Self { source_path }
    }

    fn source_dir(&self) -> &Path {
        self.source_path.as_path()
    }
}

impl MirrorFs {
    mirror_fs_readonly_methods!();
    mirror_fs_readwrite_methods!();
    file_descriptor_handler_readonly_methods!(std::path::PathBuf);
    file_descriptor_handler_readwrite_methods!(std::path::PathBuf);

    pub fn readdirplus<'a>(
        &self,
        _req: &RequestInfo,
        file_id: std::path::PathBuf,
        _file_handle: BorrowedFileHandle<'a>,
    ) -> FuseResult<Vec<(std::ffi::OsString, FileAttribute)>> {
        mirror_readdirplus(&self.source_path, file_id)
    }

    pub fn bmap(
        &self,
        _req: &RequestInfo,
        file_id: std::path::PathBuf,
        blocksize: u32,
        index: u64,
    ) -> FuseResult<u64> {
        unix_fs::bmap(&self.source_path.join(file_id), blocksize, index)
    }
}

/// Read-only mirror of a source directory.
///
/// Provides the read methods listed in the module documentation. It does not
/// reject changes made through other methods in your handler; configure a
/// read-only mount when the whole filesystem must be read-only.
pub struct MirrorFsReadOnly {
    source_path: std::path::PathBuf,
}

impl MirrorFsTrait for MirrorFsReadOnly {
    fn new(source_path: std::path::PathBuf) -> Self {
        Self { source_path }
    }

    fn source_dir(&self) -> &Path {
        self.source_path.as_path()
    }
}

impl MirrorFsReadOnly {
    mirror_fs_readonly_methods!();
    file_descriptor_handler_readonly_methods!(std::path::PathBuf);

    pub fn readdirplus<'a>(
        &self,
        _req: &RequestInfo,
        file_id: std::path::PathBuf,
        _file_handle: BorrowedFileHandle<'a>,
    ) -> FuseResult<Vec<(std::ffi::OsString, FileAttribute)>> {
        mirror_readdirplus(&self.source_path, file_id)
    }

    pub fn bmap(
        &self,
        _req: &RequestInfo,
        file_id: std::path::PathBuf,
        blocksize: u32,
        index: u64,
    ) -> FuseResult<u64> {
        unix_fs::bmap(&self.source_path.join(file_id), blocksize, index)
    }
}

/// Async mirror of a source directory.
///
/// With Linux feature `io_uring`, descriptor-backed `read`, `write`, `flush`,
/// `fsync`, and `fallocate` use io_uring. Other filesystem operations remain
/// synchronous. On BSD/macOS, all methods use the synchronous implementation.
#[cfg(feature = "async")]
pub struct MirrorFsAsync {
    source_path: std::path::PathBuf,
    #[cfg(all(target_os = "linux", feature = "io_uring"))]
    io_uring: unix_fs::io_uring::IoUringExecutor,
}

#[cfg(feature = "async")]
impl MirrorFsTrait for MirrorFsAsync {
    fn new(source_path: std::path::PathBuf) -> Self {
        Self {
            source_path,
            #[cfg(all(target_os = "linux", feature = "io_uring"))]
            io_uring: unix_fs::io_uring::IoUringExecutor::new(),
        }
    }

    fn source_dir(&self) -> &Path {
        self.source_path.as_path()
    }
}

#[cfg(feature = "async")]
impl MirrorFsAsync {
    mirror_fs_readonly_methods!(async);
    mirror_fs_readwrite_methods!(async);
    file_descriptor_handler_readonly_methods!(std::path::PathBuf, async);
    file_descriptor_handler_readwrite_methods!(std::path::PathBuf, async);

    pub async fn readdirplus<'a>(
        &self,
        _req: &RequestInfo,
        file_id: std::path::PathBuf,
        _file_handle: BorrowedFileHandle<'a>,
    ) -> FuseResult<Vec<(std::ffi::OsString, FileAttribute)>> {
        mirror_readdirplus(&self.source_path, file_id)
    }

    pub async fn bmap(
        &self,
        _req: &RequestInfo,
        file_id: std::path::PathBuf,
        blocksize: u32,
        index: u64,
    ) -> FuseResult<u64> {
        unix_fs::bmap(&self.source_path.join(file_id), blocksize, index)
    }
}

/// Async read-only mirror of a source directory.
///
/// With Linux feature `io_uring`, descriptor-backed `read`, `flush`, and
/// `fsync` use io_uring. Other filesystem operations remain synchronous; all
/// methods are synchronous internally on BSD/macOS.
#[cfg(feature = "async")]
pub struct MirrorFsReadOnlyAsync {
    source_path: std::path::PathBuf,
    #[cfg(all(target_os = "linux", feature = "io_uring"))]
    io_uring: unix_fs::io_uring::IoUringExecutor,
}

#[cfg(feature = "async")]
impl MirrorFsTrait for MirrorFsReadOnlyAsync {
    fn new(source_path: std::path::PathBuf) -> Self {
        Self {
            source_path,
            #[cfg(all(target_os = "linux", feature = "io_uring"))]
            io_uring: unix_fs::io_uring::IoUringExecutor::new(),
        }
    }

    fn source_dir(&self) -> &Path {
        self.source_path.as_path()
    }
}

#[cfg(feature = "async")]
impl MirrorFsReadOnlyAsync {
    mirror_fs_readonly_methods!(async);
    file_descriptor_handler_readonly_methods!(std::path::PathBuf, async);

    pub async fn readdirplus<'a>(
        &self,
        _req: &RequestInfo,
        file_id: std::path::PathBuf,
        _file_handle: BorrowedFileHandle<'a>,
    ) -> FuseResult<Vec<(std::ffi::OsString, FileAttribute)>> {
        mirror_readdirplus(&self.source_path, file_id)
    }

    pub async fn bmap(
        &self,
        _req: &RequestInfo,
        file_id: std::path::PathBuf,
        blocksize: u32,
        index: u64,
    ) -> FuseResult<u64> {
        unix_fs::bmap(&self.source_path.join(file_id), blocksize, index)
    }
}
