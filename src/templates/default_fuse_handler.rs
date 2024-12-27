use std::{
    ffi::{OsStr, OsString},
    path::Path,
    time::Duration,
};

use fuser::KernelConfig;

use crate::prelude::*;

/**
# DefaultFuseHandler

A default skeleton implementation for a FUSE (Filesystem in Userspace) handler. This struct provides a basic framework for implementing a custom filesystem.

## Overview

The `DefaultFuseHandler` implements the `FuseHandler` trait, providing default implementations for all FUSE operations. Most of these default implementations will return a "Not Implemented" error or panic, depending on the configuration.

## Default Implementations

The following functions are implemented with default responses, so they don't need to be explicitly implemented in derived handlers:

- `init`: Returns `Ok(())`.
- `opendir`: Returns a `FileHandle` with value 0 and empty `FUSEOpenResponseFlags`.
- `releasedir`: Returns `Ok(())`.
- `fsyncdir`: Returns `Ok(())`.
- `statfs`: Returns `StatFs::default()`.

## Usage

To use this handler, either:

1. Compose it with a more specific implementation, such as `MirrorFs`, which can use `DefaultFuseHandler` as its inner handler.
2. Use it as a reference for implementing your own `FuseHandler`.

## Configuration

The `DefaultFuseHandler` can be configured to either return errors or panic when unimplemented methods are called:

- `DefaultFuseHandler::new()`: Creates a handler that returns "Not Implemented" errors.
- `DefaultFuseHandler::new_with_panic()`: Creates a handler that panics on unimplemented methods.

## Note

This is a basic skeleton. For more complete implementations, refer to the templates provided in the library.
*/
pub struct DefaultFuseHandler {
    handling: HandlingMethod,
}

enum HandlingMethod {
    Panic,
    Error(ErrorKind),
}

impl DefaultFuseHandler {
    /// Creates a new `DefaultFuseHandler` that returns "Not Implemented" errors for each unimplemented FUSE call.
    ///
    /// This is useful for gradually implementing FUSE operations, as it allows the filesystem to
    /// function (albeit with limited capabilities) even when not all operations are implemented.
    pub fn new() -> Self {
        DefaultFuseHandler {
            handling: HandlingMethod::Error(ErrorKind::FunctionNotImplemented),
        }
    }

    /// Creates a new `DefaultFuseHandler` that panics for each unimplemented FUSE call.
    ///
    /// This is useful for debugging purposes, as it immediately highlights which FUSE operations
    /// are being called but not yet implemented.
    pub fn new_with_panic() -> Self {
        DefaultFuseHandler {
            handling: HandlingMethod::Panic,
        }
    }

    /// Creates a new `DefaultFuseHandler` that returns a custom error for each unimplemented FUSE call.
    ///
    /// This is useful to give a different message for the user, like PermissionDenied.
    pub fn new_with_custom_error(error_kind: ErrorKind) -> Self {
        DefaultFuseHandler {
            handling: HandlingMethod::Error(error_kind),
        }
    }
}

impl<TId: FileIdType> FuseHandler<TId> for DefaultFuseHandler {
    fn get_inner(&self) -> &dyn FuseHandler<TId> {
        panic!("Base Fuse don't have inner type")
    }

    fn get_default_ttl(&self) -> Duration {
        Duration::from_secs(1)
    }

    fn init(&self, _req: &RequestInfo, _config: &mut KernelConfig) -> FuseResult<()> {
        Ok(())
    }

    fn destroy(&self) {}

    fn lookup(
        &self,
        _req: &RequestInfo,
        parent_id: TId,
        name: &OsStr,
    ) -> FuseResult<TId::Metadata> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "lookup(parent_file: {}, name {:?})",
                        parent_id.display(),
                        Path::display(name.as_ref())
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!("[Not Implemented] lookup"),
        }
    }

    fn forget(&self, _req: &RequestInfo, _file_id: TId, _nlookup: u64) {}

    fn getattr(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: Option<FileHandle>,
    ) -> FuseResult<FileAttribute> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "getattr(file_id: {}, file_handle: {:?})",
                        file_id.display(),
                        file_handle
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] getattr(file_id: {}, file_handle: {:?})",
                file_id.display(),
                file_handle
            ),
        }
    }

    fn setattr(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        attrs: SetAttrRequest,
    ) -> FuseResult<FileAttribute> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "setattr(file_id: {}, attrs: {:?})",
                        file_id.display(),
                        attrs
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] setattr(file_id: {}, attrs: {:?})",
                file_id.display(),
                attrs
            ),
        }
    }

    fn readlink(&self, _req: &RequestInfo, file_id: TId) -> FuseResult<Vec<u8>> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!("readlink(file_id: {})", file_id.display())
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => {
                panic!("[Not Implemented] readlink(file_id: {})", file_id.display())
            }
        }
    }

    fn mknod(
        &self,
        _req: &RequestInfo,
        parent_id: TId,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: DeviceType,
    ) -> FuseResult<TId::Metadata> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(
                PosixError::new(kind, if cfg!(debug_assertions) {
                    format!(
                    "mknod(parent_id: {}, name: {:?}, mode: {}, umask: {}, rdev: {:?})",
                    parent_id.display(),
                    Path::new(name),
                    mode,
                    umask,
                    rdev
                )
        } else {
            String::new()
        })
    ),
    HandlingMethod::Panic => panic!(
                "[Not Implemented] mknod(parent_id: {}, name: {:?}, mode: {}, umask: {}, rdev: {:?})",
                parent_id.display(),
                Path::new(name),
                mode,
                umask,
                rdev
            ),
        }
    }

    fn mkdir(
        &self,
        _req: &RequestInfo,
        parent_id: TId,
        name: &OsStr,
        mode: u32,
        umask: u32,
    ) -> FuseResult<TId::Metadata> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "mkdir(parent_id: {}, name: {:?}, mode: {}, umask: {})",
                        parent_id.display(),
                        Path::new(name),
                        mode,
                        umask
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] mkdir(parent_id: {}, name: {:?}, mode: {}, umask: {})",
                parent_id.display(),
                Path::new(name),
                mode,
                umask
            ),
        }
    }

    fn unlink(&self, _req: &RequestInfo, parent_id: TId, name: &OsStr) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "unlink(parent_id: {}, name: {:?})",
                        parent_id.display(),
                        Path::new(name)
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] unlink(parent_id: {}, name: {:?})",
                parent_id.display(),
                Path::new(name)
            ),
        }
    }

    fn rmdir(&self, _req: &RequestInfo, parent_id: TId, name: &OsStr) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "rmdir(parent_id: {}, name: {:?})",
                        parent_id.display(),
                        Path::new(name)
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] rmdir(parent_id: {}, name: {:?})",
                parent_id.display(),
                Path::new(name)
            ),
        }
    }

    fn symlink(
        &self,
        _req: &RequestInfo,
        parent_id: TId,
        link_name: &OsStr,
        target: &Path,
    ) -> FuseResult<TId::Metadata> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "symlink(parent_id: {}, link_name: {:?}, target: {:?})",
                        parent_id.display(),
                        Path::new(link_name),
                        target
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] symlink(parent_id: {}, link_name: {:?}, target: {:?})",
                parent_id.display(),
                Path::new(link_name),
                target
            ),
        }
    }

    fn rename(
        &self,
        _req: &RequestInfo,
        parent_id: TId,
        name: &OsStr,
        newparent: TId,
        newname: &OsStr,
        flags: RenameFlags,
    ) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(
                PosixError::new(kind, if cfg!(debug_assertions) {
                    format!(
                    "rename(parent_id: {}, name: {:?}, newparent: {}, newname: {:?}, flags: {:?})",
                    parent_id.display(),
                    Path::new(name),
                    newparent.display(),
                    Path::new(newname),
                    flags
                )
        } else {
            String::new()
        })
    ),
    HandlingMethod::Panic => panic!(
                "[Not Implemented] rename(parent_id: {}, name: {:?}, newparent: {}, newname: {:?}, flags: {:?})",
                parent_id.display(),
                Path::new(name),
                newparent.display(),
                Path::new(newname),
                flags
            ),
        }
    }

    fn link(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        newparent: TId,
        newname: &OsStr,
    ) -> FuseResult<TId::Metadata> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "link(file_id: {}, newparent: {}, newname: {:?})",
                        file_id.display(),
                        newparent.display(),
                        Path::new(newname)
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] link(file_id: {}, newparent: {}, newname: {:?})",
                file_id.display(),
                newparent.display(),
                Path::new(newname)
            ),
        }
    }

    fn open(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!("open(file_id: {}, flags: {:?})", file_id.display(), flags)
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] open(file_id: {}, flags: {:?})",
                file_id.display(),
                flags
            ),
        }
    }

    fn read(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: FileHandle,
        seek: SeekFrom,
        size: u32,
        flags: FUSEOpenFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(
                PosixError::new(kind, if cfg!(debug_assertions) {
                    format!(
                    "read(file_id: {}, file_handle: {:?}, seek: {:?}, size: {}, flags: {:?}, lock_owner: {:?})",
                    file_id.display(),
                    file_handle,
                    seek,
                    size,
                    flags,
                    lock_owner
                )
        } else {
            String::new()
        })
    ),
    HandlingMethod::Panic => panic!(
                "[Not Implemented] read(file_id: {}, file_handle: {:?}, seek: {:?}, size: {}, flags: {:?}, lock_owner: {:?})",
                file_id.display(),
                file_handle,
                seek,
                size,
                flags,
                lock_owner
            ),
        }
    }

    fn write(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: FileHandle,
        seek: SeekFrom,
        data: Vec<u8>,
        write_flags: FUSEWriteFlags,
        flags: OpenFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<u32> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(
                PosixError::new(kind, if cfg!(debug_assertions) {
                    format!(
                    "write(file_id: {}, file_handle: {:?}, seek: {:?}, data_len: {}, write_flags: {:?}, flags: {:?}, lock_owner: {:?})",
                    file_id.display(),
                    file_handle,
                    seek,
                    data.len(),
                    write_flags,
                    flags,
                    lock_owner
                )
        } else {
            String::new()
        })
    ),
    HandlingMethod::Panic => panic!(
                "[Not Implemented] write(file_id: {}, file_handle: {:?}, seek: {:?}, data_len: {}, write_flags: {:?}, flags: {:?}, lock_owner: {:?})",
                file_id.display(),
                file_handle,
                seek,
                data.len(),
                write_flags,
                flags,
                lock_owner
            ),
        }
    }

    fn flush(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: FileHandle,
        lock_owner: u64,
    ) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "flush(file_id: {}, file_handle: {:?}, lock_owner: {})",
                        file_id.display(),
                        file_handle,
                        lock_owner
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] flush(file_id: {}, file_handle: {:?}, lock_owner: {})",
                file_id.display(),
                file_handle,
                lock_owner
            ),
        }
    }

    fn release(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: FileHandle,
        flags: OpenFlags,
        lock_owner: Option<u64>,
        flush: bool,
    ) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(
                PosixError::new(kind, if cfg!(debug_assertions) {
                    format!(
                    "release(file_id: {}, file_handle: {:?}, flags: {:?}, lock_owner: {:?}, flush: {})",
                    file_id.display(),
                    file_handle,
                    flags,
                    lock_owner,
                    flush
                )
        } else {
            String::new()
        })
    ),
    HandlingMethod::Panic => panic!(
                "[Not Implemented] release(file_id: {}, file_handle: {:?}, flags: {:?}, lock_owner: {:?}, flush: {})",
                file_id.display(),
                file_handle,
                flags,
                lock_owner,
                flush
            ),
        }
    }

    fn fsync(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: FileHandle,
        datasync: bool,
    ) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "fsync(file_id: {}, file_handle: {:?}, datasync: {})",
                        file_id.display(),
                        file_handle,
                        datasync
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] fsync(file_id: {}, file_handle: {:?}, datasync: {})",
                file_id.display(),
                file_handle,
                datasync
            ),
        }
    }

    fn opendir(
        &self,
        _req: &RequestInfo,
        _file_id: TId,
        _flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        Ok((FileHandle::from(0), FUSEOpenResponseFlags::empty()))
    }

    fn readdir(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: FileHandle,
    ) -> FuseResult<Vec<(OsString, TId::MinimalMetadata)>> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "readdir(file_id: {}, file_handle: {:?})",
                        file_id.display(),
                        file_handle
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] readdir(file_id: {}, file_handle: {:?})",
                file_id.display(),
                file_handle
            ),
        }
    }

    fn readdirplus(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: FileHandle,
    ) -> FuseResult<Vec<(OsString, TId::Metadata)>> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "readdirplus(file_id: {}, file_handle: {:?})",
                        file_id.display(),
                        file_handle
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] readdirplus(file_id: {}, file_handle: {:?})",
                file_id.display(),
                file_handle
            ),
        }
    }

    fn releasedir(
        &self,
        _req: &RequestInfo,
        _file_id: TId,
        _file_handle: FileHandle,
        _flags: OpenFlags,
    ) -> FuseResult<()> {
        Ok(())
    }

    fn fsyncdir(
        &self,
        _req: &RequestInfo,
        _file_id: TId,
        _file_handle: FileHandle,
        _datasync: bool,
    ) -> FuseResult<()> {
        Ok(())
    }

    fn statfs(&self, _req: &RequestInfo, _file_id: TId) -> FuseResult<StatFs> {
        Ok(StatFs::default())
    }

    fn setxattr(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        name: &OsStr,
        _value: Vec<u8>,
        flags: FUSESetXAttrFlags,
        position: u32,
    ) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "setxattr(file_id: {}, name: {:?}, flags: {:?}, position: {})",
                        file_id.display(),
                        name,
                        flags,
                        position
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] setxattr(file_id: {}, name: {:?}, flags: {:?}, position: {})",
                file_id.display(),
                name,
                flags,
                position
            ),
        }
    }

    fn getxattr(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        name: &OsStr,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "getxattr(file_id: {}, name: {:?}, size: {})",
                        file_id.display(),
                        name,
                        size
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] getxattr(file_id: {}, name: {:?}, size: {})",
                file_id.display(),
                name,
                size
            ),
        }
    }

    fn listxattr(&self, _req: &RequestInfo, file_id: TId, size: u32) -> FuseResult<Vec<u8>> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!("listxattr(file_id: {}, size: {})", file_id.display(), size)
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] listxattr(file_id: {}, size: {})",
                file_id.display(),
                size
            ),
        }
    }

    fn removexattr(&self, _req: &RequestInfo, file_id: TId, name: &OsStr) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "removexattr(file_id: {}, name: {:?})",
                        file_id.display(),
                        name
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] removexattr(file_id: {}, name: {:?})",
                file_id.display(),
                name
            ),
        }
    }

    fn access(&self, _req: &RequestInfo, file_id: TId, mask: AccessMask) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!("access(file_id: {}, mask: {:?})", file_id.display(), mask)
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] access(file_id: {}, mask: {:?})",
                file_id.display(),
                mask
            ),
        }
    }

    fn getlk(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
    ) -> FuseResult<LockInfo> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(
                PosixError::new(kind, if cfg!(debug_assertions) {
                    format!(
                    "getlk(file_id: {}, file_handle: {:?}, lock_owner: {}, lock_info: {:?})",
                    file_id.display(),
                    file_handle,
                    lock_owner,
                    lock_info
                )
        } else {
            String::new()
        })
    ),
    HandlingMethod::Panic => panic!(
                "[Not Implemented] getlk(file_id: {}, file_handle: {:?}, lock_owner: {}, lock_info: {:?})",
                file_id.display(),
                file_handle,
                lock_owner,
                lock_info
            ),
        }
    }

    fn setlk(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
        sleep: bool,
    ) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(
                PosixError::new(kind, if cfg!(debug_assertions) {
                    format!(
                    "setlk(file_id: {}, file_handle: {:?}, lock_owner: {}, lock_info: {:?}, sleep: {})",
                    file_id.display(),
                    file_handle,
                    lock_owner,
                    lock_info,
                    sleep
                )
        } else {
            String::new()
        })
    ),
    HandlingMethod::Panic => panic!(
                "[Not Implemented] setlk(file_id: {}, file_handle: {:?}, lock_owner: {}, lock_info: {:?}, sleep: {})",
                file_id.display(),
                file_handle,
                lock_owner,
                lock_info,
                sleep
            ),
        }
    }

    fn bmap(&self, _req: &RequestInfo, file_id: TId, blocksize: u32, idx: u64) -> FuseResult<u64> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "bmap(file_id: {}, blocksize: {}, idx: {})",
                        file_id.display(),
                        blocksize,
                        idx
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] bmap(file_id: {}, blocksize: {}, idx: {})",
                file_id.display(),
                blocksize,
                idx
            ),
        }
    }

    fn ioctl(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: FileHandle,
        flags: IOCtlFlags,
        cmd: u32,
        in_data: Vec<u8>,
        out_size: u32,
    ) -> FuseResult<(i32, Vec<u8>)> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "ioctl(file_id: {}, file_handle: {:?}, flags: {:?}, cmd: {}, in_data: {:?}, out_size: {})",
                        file_id.display(),
                        file_handle,
                        flags,
                        cmd,
                        in_data,
                        out_size
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] ioctl(file_id: {}, file_handle: {:?}, flags: {:?}, cmd: {}, in_data: {:?}, out_size: {})",
                file_id.display(),
                file_handle,
                flags,
                cmd,
                in_data,
                out_size
            ),
        }
    }

    fn create(
        &self,
        _req: &RequestInfo,
        parent_id: TId,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, TId::Metadata, FUSEOpenResponseFlags)> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "create(parent_id: {}, name: {:?}, mode: {}, umask: {}, flags: {:?})",
                        parent_id.display(),
                        name,
                        mode,
                        umask,
                        flags
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] create(parent_id: {}, name: {:?}, mode: {}, umask: {}, flags: {:?})",
                parent_id.display(),
                name,
                mode,
                umask,
                flags
            ),
        }
    }

    fn fallocate(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: FileHandle,
        offset: i64,
        length: i64,
        mode: i32,
    ) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(
                PosixError::new(kind, if cfg!(debug_assertions) {
                    format!(
                        "fallocate(file_id: {}, file_handle: {:?}, offset: {}, length: {}, mode: {})",
                        file_id.display(),
                        file_handle,
                        offset,
                        length,
                        mode
                    )
                } else {
                    String::new()
                })
            ),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] fallocate(file_id: {}, file_handle: {:?}, offset: {}, length: {}, mode: {})",
                file_id.display(),
                file_handle,
                offset,
                length,
                mode
            ),
        }
    }

    fn lseek(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: FileHandle,
        seek: SeekFrom,
    ) -> FuseResult<i64> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "lseek(file_id: {}, file_handle: {:?}, seek: {:?})",
                        file_id.display(),
                        file_handle,
                        seek
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] lseek(file_id: {}, file_handle: {:?}, seek: {:?})",
                file_id.display(),
                file_handle,
                seek
            ),
        }
    }

    fn copy_file_range(
        &self,
        _req: &RequestInfo,
        file_in: TId,
        file_handle_in: FileHandle,
        offset_in: i64,
        file_out: TId,
        file_handle_out: FileHandle,
        offset_out: i64,
        len: u64,
        flags: u32, // Not implemented yet in standard
    ) -> FuseResult<u32> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(
                PosixError::new(kind, if cfg!(debug_assertions) {
                    format!(
                        "copy_file_range(file_in: {}, file_handle_in: {:?}, offset_in: {}, file_out: {}, file_handle_out: {:?}, offset_out: {}, len: {}, flags: {})",
                        file_in.display(),
                        file_handle_in,
                        offset_in,
                        file_out.display(),
                        file_handle_out,
                        offset_out,
                        len,
                        flags
                    )
                } else {
                    String::new()
                })
            ),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] copy_file_range(file_in: {}, file_handle_in: {:?}, offset_in: {}, file_out: {}, file_handle_out: {:?}, offset_out: {}, len: {}, flags: {})",
                file_in.display(),
                file_handle_in,
                offset_in,
                file_out.display(),
                file_handle_out,
                offset_out,
                len,
                flags
            ),
        }
    }
}
