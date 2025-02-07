/*!
# MirrorFs

A FUSE (Filesystem in Userspace) handler that mirrors the content of another folder in either read-only or read-write mode.

## Overview

The `MirrorFs` struct implements the `FuseHandler` trait, providing a way to create a mirror of an existing filesystem. It comes in two variants:

1. `MirrorFsReadOnly`: A read-only version that only allows read operations on the mirrored content.
2. `MirrorFs`: A read-write version that allows both read and write operations on the mirrored content.

## Implementation Details

- Both variants use a `PathBuf` to represent the repository path they're mirroring.
- They wrap another `FuseHandler<PathBuf>` implementation, allowing for composition of filesystem behaviors.
- Most FUSE operations are implemented by translating paths and delegating to the `unix_fs` module.
- The implementation uses macros to define common methods for both read-only and read-write variants.

## Usage

To use these handlers:

1. Create a new `MirrorFsReadOnly` or `MirrorFs` instance by providing a repository path and an inner `FuseHandler<PathBuf>` implementation:

   ```text
   let read_only_fs = MirrorFsReadOnly::new(repo_path, inner_handler);
   // or
   let read_write_fs = MirrorFs::new(repo_path, inner_handler);
   ```

2. Use the resulting MirrorFsReadOnly or MirrorFs as your FUSE handler.

3. Alternatively, you can use MirrorFs or MirrorFsReadOnly as delegators in your own FUSE implementation (see FuseHandler documentation for more details).

## Unimplemented Functions
The following FUSE operations are not implemented in either variant:

- link
- setlk
- getlk
- bmap
- ioctl


## Important Note
This implementation does not include safeguards against recursive mounting scenarios. Users should be cautious when choosing mount points to avoid potential system hangs.

For example, if the MirrorFs is set up like this:
```text
let fs = MirrorFs::new("/my_repo");
mount(fs, "/my_repo/mountpoint")
```

Operations like ls /my_repo/mountpoint could cause the system to hang indefinitely. This occurs because the filesystem would repeatedly try to access its own mountpoint, creating an endless loop.

Specifically, operations such as lstat (used in lookup, getattr, and ls commands) can trigger this recursive behavior when a child directory in the mirrored filesystem is also a parent in the actual filesystem hierarchy.

To avoid this issue, ensure that the mountpoint is not located within the mirrored repository.

## Read-Only vs Read-Write
- MirrorFsReadOnly: This variant only implements methods for reading and accessing file metadata. It does not allow any modifications to the mirrored filesystem.
- MirrorFs: This variant implements all methods from MirrorFsReadOnly plus additional methods for modifying the filesystem, such as creating, deleting, and modifying files and directories.


## Note
For more specific implementations or to extend functionality, you can modify these handlers or use them as a reference for implementing your own FuseHandler.

If you intend to enforce read-only at the fuse level,
prefer the usage of option `MountOption::RO` instead of `MirrorFsReadOnly`.
*/

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use fd_handler_helper::*;

use crate::prelude::*;
use crate::templates::*;
use crate::unix_fs;

macro_rules! mirror_fs_readonly_methods {
    () => {
        fn access(&self, _req: &RequestInfo, file_id: PathBuf, mask: AccessMask) -> FuseResult<()> {
            let file_path = self.source_path.join(file_id);
            unix_fs::access(&file_path, mask)
        }

        fn getattr(
            &self,
            _req: &RequestInfo,
            file_id: PathBuf,
            _file_handle: Option<BorrowedFileHandle>,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(file_id);
            let fd = unix_fs::open(file_path.as_ref(), OpenFlags::empty())?;
            let result = unix_fs::getattr(fd.as_fd());
            result
        }

        fn getxattr(
            &self,
            _req: &RequestInfo,
            file_id: PathBuf,
            name: &OsStr,
            size: u32,
        ) -> FuseResult<Vec<u8>> {
            let file_path = self.source_path.join(file_id);
            unix_fs::getxattr(&file_path, name, size)
        }

        fn listxattr(
            &self,
            _req: &RequestInfo,
            file_id: PathBuf,
            size: u32,
        ) -> FuseResult<Vec<u8>> {
            let file_path = self.source_path.join(file_id);
            unix_fs::listxattr(&file_path, size)
        }

        fn lookup(
            &self,
            _req: &RequestInfo,
            parent_id: PathBuf,
            name: &OsStr,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::lookup(&file_path)
        }

        fn open(
            &self,
            _req: &RequestInfo,
            file_id: PathBuf,
            flags: OpenFlags,
        ) -> FuseResult<(OwnedFileHandle, FUSEOpenResponseFlags)> {
            let file_path = self.source_path.join(file_id);
            let fd = unix_fs::open(file_path.as_ref(), flags)?;
            // Open by definition returns positive Fd or error
            let file_handle = OwnedFileHandle::from_owned_fd(fd).unwrap();
            Ok((file_handle, FUSEOpenResponseFlags::empty()))
        }

        fn readdir(
            &self,
            _req: &RequestInfo,
            file_id: PathBuf,
            _file_handle: BorrowedFileHandle,
        ) -> FuseResult<Vec<(OsString, FileKind)>> {
            let folder_path = self.source_path.join(file_id);
            let children = unix_fs::readdir(folder_path.as_ref())?;
            let mut result = Vec::new();
            result.push((OsString::from("."), FileKind::Directory));
            result.push((OsString::from(".."), FileKind::Directory));
            for (child_name, child_kind) in children {
                result.push((child_name, child_kind));
            }
            Ok(result)
        }

        fn readlink(&self, _req: &RequestInfo, file_id: PathBuf) -> FuseResult<Vec<u8>> {
            let file_path = self.source_path.join(file_id);
            unix_fs::readlink(&file_path)
        }

        fn statfs(&self, _req: &RequestInfo, file_id: PathBuf) -> FuseResult<StatFs> {
            let file_path = self.source_path.join(file_id);
            unix_fs::statfs(&file_path)
        }
    };
}

macro_rules! mirror_fs_readwrite_methods {
    () => {
        fn create(
            &self,
            _req: &RequestInfo,
            parent_id: PathBuf,
            name: &OsStr,
            mode: u32,
            umask: u32,
            flags: OpenFlags,
        ) -> FuseResult<(OwnedFileHandle, FileAttribute, FUSEOpenResponseFlags)> {
            let file_path = self.source_path.join(parent_id).join(name);
            let (fd, file_attr) = unix_fs::create(&file_path, mode, umask, flags)?;
            // Open by definition returns positive Fd or error
            let file_handle = OwnedFileHandle::from_owned_fd(fd).unwrap();
            Ok((file_handle, file_attr, FUSEOpenResponseFlags::empty()))
        }

        fn mkdir(
            &self,
            _req: &RequestInfo,
            parent_id: PathBuf,
            name: &OsStr,
            mode: u32,
            umask: u32,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::mkdir(&file_path, mode, umask)
        }

        fn mknod(
            &self,
            _req: &RequestInfo,
            parent_id: PathBuf,
            name: &OsStr,
            mode: u32,
            umask: u32,
            rdev: DeviceType,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::mknod(&file_path, mode, umask, rdev)
        }

        fn removexattr(
            &self,
            _req: &RequestInfo,
            file_id: PathBuf,
            name: &OsStr,
        ) -> FuseResult<()> {
            let file_path = self.source_path.join(file_id);
            unix_fs::removexattr(&file_path, name)
        }

        fn rename(
            &self,
            _req: &RequestInfo,
            parent_id: PathBuf,
            name: &OsStr,
            newparent: PathBuf,
            newname: &OsStr,
            flags: RenameFlags,
        ) -> FuseResult<()> {
            let oldpath = self.source_path.join(parent_id).join(name);
            let newpath = self.source_path.join(newparent).join(newname);
            unix_fs::rename(&oldpath, &newpath, flags)
        }

        fn rmdir(&self, _req: &RequestInfo, parent_id: PathBuf, name: &OsStr) -> FuseResult<()> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::rmdir(&file_path)
        }

        fn setattr(
            &self,
            _req: &RequestInfo,
            file_id: PathBuf,
            attrs: SetAttrRequest,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(file_id);
            unix_fs::setattr(&file_path, attrs)
        }

        fn setxattr(
            &self,
            _req: &RequestInfo,
            file_id: PathBuf,
            name: &OsStr,
            value: Vec<u8>,
            flags: FUSESetXAttrFlags,
            position: u32,
        ) -> FuseResult<()> {
            let file_path = self.source_path.join(file_id);
            unix_fs::setxattr(&file_path, name, &value, flags, position)
        }

        fn symlink(
            &self,
            _req: &RequestInfo,
            parent_id: PathBuf,
            link_name: &OsStr,
            target: &std::path::Path,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(link_name);
            unix_fs::symlink(&file_path, target)
        }

        fn unlink(&self, _req: &RequestInfo, parent_id: PathBuf, name: &OsStr) -> FuseResult<()> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::unlink(&file_path)
        }
    };
}

pub trait MirrorFsTrait: FuseHandler<PathBuf> {
    fn new<U: FuseHandler<PathBuf>>(source_path: PathBuf, inner: U) -> Self;

    fn source_dir(&self) -> &Path;
}

/// Specific documentation is located in parent module documentation.
pub struct MirrorFs {
    source_path: PathBuf,
    inner: Box<FdHandlerHelper<PathBuf>>,
}

impl MirrorFsTrait for MirrorFs {
    fn new<U: FuseHandler<PathBuf>>(source_path: PathBuf, inner: U) -> Self {
        Self {
            source_path,
            inner: Box::new(FdHandlerHelper::new(inner)),
        }
    }

    fn source_dir(&self) -> &Path {
        self.source_path.as_path()
    }
}

impl FuseHandler<PathBuf> for MirrorFs {
    fn get_inner(&self) -> &dyn FuseHandler<PathBuf> {
        self.inner.as_ref()
    }

    mirror_fs_readonly_methods!();
    mirror_fs_readwrite_methods!();
}

/// Specific documentation is located in parent module documentation.
pub struct MirrorFsReadOnly {
    source_path: PathBuf,
    inner: Box<FdHandlerHelperReadOnly<PathBuf>>,
}

impl MirrorFsTrait for MirrorFsReadOnly {
    fn new<THandler: FuseHandler<PathBuf>>(source_path: PathBuf, inner: THandler) -> Self {
        Self {
            source_path,
            inner: Box::new(FdHandlerHelperReadOnly::new(inner)),
        }
    }

    fn source_dir(&self) -> &Path {
        self.source_path.as_path()
    }
}

impl FuseHandler<PathBuf> for MirrorFsReadOnly {
    fn get_inner(&self) -> &dyn FuseHandler<PathBuf> {
        self.inner.as_ref()
    }

    mirror_fs_readonly_methods!();
}
