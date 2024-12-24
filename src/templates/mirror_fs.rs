use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use crate::prelude::*;
use crate::templates::FdHandlerHelper;

/**
# MirrorFs

A FUSE (Filesystem in Userspace) handler that mirrors the content of another folder in a read-write manner.

## Overview

The `MirrorFs` struct implements the `FuseHandler` trait, providing a way to create a mirror of an existing filesystem. It allows read and write operations on the mirrored content.

## Implementation Details

- `MirrorFs` uses a `PathBuf` to represent the repository path it's mirroring.
- It wraps another `FuseHandler<PathBuf>` implementation, allowing for composition of filesystem behaviors.
- Most FUSE operations are implemented by translating paths and delegating to the `posix_fs` module.

## Usage

To use this handler:

1. Create a new `MirrorFs` instance by providing a repository path and an inner `FuseHandler<PathBuf>` implementation:

   'MirrorFs::new(repo_path, inner_handler)'

2. Use the resulting `MirrorFs` as your FUSE handler.

## Unimplemented Functions

The following FUSE operations are not implemented in `MirrorFs`:

- 'link'
- 'setlk'
- 'getlk'
- 'bmap'
- 'ioctl'

## Important Note

This implementation does not include safeguards against recursive mounting scenarios. Users should be cautious when choosing mount points to avoid potential system hangs.

For example, if the MirrorFs is set up like this:

```text
let fs = MirrorFs::new("/my_repo");
mount(fs, "/my_repo/mountpoint")
```

Operations like 'ls /my_repo/mountpoint' could cause the system to hang indefinitely. This occurs because the filesystem would repeatedly try to access its own mountpoint, creating an endless loop.

Specifically, operations such as 'lstat' (used in lookup, getattr, and ls commands) can trigger this recursive behavior when a child directory in the mirrored filesystem is also a parent in the actual filesystem hierarchy.

To avoid this issue, ensure that the mountpoint is not located within the mirrored repository.


## Note

For more specific implementations or to extend functionality, you can modify this handler or use it as a reference for implementing your own `FuseHandler`.
*/

pub struct MirrorFs {
    source_path: PathBuf,
    inner: Box<dyn FuseHandler<PathBuf>>,
}

impl MirrorFs {
    pub fn new<T: FuseHandler<PathBuf>>(source_path: PathBuf, inner: T) -> Self {
        Self {
            source_path,
            inner: Box::new(FdHandlerHelper::new(inner)),
        }
    }
}

impl FuseHandler<PathBuf> for MirrorFs {
    fn get_inner(&self) -> &Box<(dyn FuseHandler<PathBuf>)> {
        &self.inner
    }

    fn lookup(
        &self,
        _req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
    ) -> FuseResult<FileAttribute> {
        let file_path = self.source_path.join(parent_id).join(name);
        posix_fs::lookup(&file_path)
    }

    fn getattr(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        _file_handle: Option<FileHandle>,
    ) -> FuseResult<FileAttribute> {
        let file_path = self.source_path.join(file_id);
        let fd = posix_fs::open(file_path.as_ref(), OpenFlags::empty())?;
        let result = posix_fs::getattr(&fd);
        result
    }

    fn setattr(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        attrs: SetAttrRequest,
    ) -> FuseResult<FileAttribute> {
        let file_path = self.source_path.join(file_id);
        posix_fs::setattr(&file_path, attrs)
    }

    fn readlink(&self, _req: &RequestInfo, file_id: PathBuf) -> FuseResult<Vec<u8>> {
        let file_path = self.source_path.join(file_id);
        posix_fs::readlink(&file_path)
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
        posix_fs::mknod(&file_path, mode, umask, rdev)
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
        posix_fs::mkdir(&file_path, mode, umask)
    }

    fn unlink(&self, _req: &RequestInfo, parent_id: PathBuf, name: &OsStr) -> FuseResult<()> {
        let file_path = self.source_path.join(parent_id).join(name);
        posix_fs::unlink(&file_path)
    }

    fn rmdir(&self, _req: &RequestInfo, parent_id: PathBuf, name: &OsStr) -> FuseResult<()> {
        let file_path = self.source_path.join(parent_id).join(name);
        posix_fs::rmdir(&file_path)
    }

    fn symlink(
        &self,
        _req: &RequestInfo,
        parent_id: PathBuf,
        link_name: &OsStr,
        target: &std::path::Path,
    ) -> FuseResult<FileAttribute> {
        let file_path = self.source_path.join(parent_id).join(link_name);
        posix_fs::symlink(&file_path, target)
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
        posix_fs::rename(&oldpath, &newpath, flags)
    }

    fn open(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        let file_path = self.source_path.join(file_id);
        let mut fd = posix_fs::open(file_path.as_ref(), flags)?;
        Ok((fd.take_to_file_handle()?, FUSEOpenResponseFlags::empty()))
    }
    fn readdir(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        _file_handle: FileHandle,
    ) -> FuseResult<Vec<(OsString, FileKind)>> {
        let folder_path = self.source_path.join(file_id);
        let children = posix_fs::readdir(folder_path.as_ref())?;
        let mut result = Vec::new();
        result.push((OsString::from("."), FileKind::Directory));
        result.push((OsString::from(".."), FileKind::Directory));
        for (child_name, child_kind) in children {
            result.push((child_name, child_kind));
        }
        Ok(result)
    }

    fn statfs(&self, _req: &RequestInfo, file_id: PathBuf) -> FuseResult<StatFs> {
        let file_path = self.source_path.join(file_id);
        posix_fs::statfs(&file_path)
    }

    fn setxattr(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        name: &OsStr,
        value: Vec<u8>,
        _flags: FUSESetXAttrFlags,
        position: u32,
    ) -> FuseResult<()> {
        let file_path = self.source_path.join(file_id);
        posix_fs::setxattr(&file_path, name, &value, position)
    }

    fn getxattr(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        name: &OsStr,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        let file_path = self.source_path.join(file_id);
        posix_fs::getxattr(&file_path, name, size)
    }

    fn listxattr(&self, _req: &RequestInfo, file_id: PathBuf, size: u32) -> FuseResult<Vec<u8>> {
        let file_path = self.source_path.join(file_id);
        posix_fs::listxattr(&file_path, size)
    }

    fn access(&self, _req: &RequestInfo, file_id: PathBuf, mask: AccessMask) -> FuseResult<()> {
        let file_path = self.source_path.join(file_id);
        posix_fs::access(&file_path, mask)
    }

    fn create(
        &self,
        _req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FileAttribute, FUSEOpenResponseFlags)> {
        let file_path = self.source_path.join(parent_id).join(name);
        let (mut fd, file_attr) = posix_fs::create(&file_path, mode, umask, flags)?;
        Ok((
            fd.take_to_file_handle()?,
            file_attr,
            FUSEOpenResponseFlags::empty(),
        ))
    }
}
