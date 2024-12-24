use crate::posix_fs;
use crate::prelude::*;

/**
# FdHandlerHelper

A helper implementation for a FUSE (Filesystem in Userspace) handler that manages file operations using file descriptors.

## Overview

The `FdHandlerHelper<T>` implements the `FuseHandler<T>` trait, providing implementations for file operations that rely on file handles. It assumes that file handles represent file descriptors on the filesystem.

## Implementation Details

This helper implements the following `FuseHandler<T>` methods:

- `read`: Reads data from a file using the file descriptor.
- `write`: Writes data to a file using the file descriptor.
- `flush`: Flushes the file associated with the file descriptor.
- `release`: Releases (closes) the file descriptor.
- `fsync`: Synchronizes the file's in-core state with storage device.
- `fallocate`: Manipulates the allocated disk space for the file.
- `lseek`: Repositions the file offset of the file descriptor.
- `copy_file_range`: Copies a range of data from one file to another.

## Usage

To use this helper:

1. Create an instance of `FdHandlerHelper<T>` by passing an inner `FuseHandler<T>` implementation.
2. Use it as the primary `FuseHandler<T>` in your FUSE filesystem implementation.

## Important Considerations

When implementing the `open` and `create` methods in your filesystem:

- Ensure that the returned file handle can be converted to a valid file descriptor.
- The file handle should represent an open file descriptor on the underlying filesystem.

## Example

```text
let inner_handler = YourInnerHandler::new();
let fd_handler = FdHandlerHelper::new(inner_handler);
// Use fd_handler as your primary FuseHandler
```
## Note
This helper provides a convenient way to implement file operations using POSIX-like file descriptors. It's particularly useful for filesystems that interact with an underlying local filesystem.
*/

pub struct FdHandlerHelper<T: FileIdType> {
    inner: Box<dyn FuseHandler<T>>,
}

impl<T: FileIdType> FdHandlerHelper<T> {
    pub fn new<U: FuseHandler<T>>(inner: U) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
}

impl<T: FileIdType> FuseHandler<T> for FdHandlerHelper<T> {
    fn get_inner(&self) -> &Box<(dyn FuseHandler<T>)> {
        &self.inner
    }

    fn read(
        &self,
        _req: &RequestInfo,
        _file_id: T,
        file_handle: FileHandle,
        offset: i64,
        size: u32,
        _flags: FUSEOpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => posix_fs::read(&fd, offset, size),
            Err(e) => Err(e.into()),
        }
    }

    fn write(
        &self,
        _req: &RequestInfo,
        _file_id: T,
        file_handle: FileHandle,
        offset: i64,
        data: Vec<u8>,
        _write_flags: FUSEWriteFlags,
        _flags: OpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<u32> {
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => posix_fs::write(&fd, offset, &data),
            Err(e) => Err(e.into()),
        }
    }

    fn flush(
        &self,
        _req: &RequestInfo,
        _file_id: T,
        file_handle: FileHandle,
        _lock_owner: u64,
    ) -> FuseResult<()> {
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => posix_fs::flush(&fd),
            Err(e) => Err(e.into()),
        }
    }

    fn release(
        &self,
        _req: &RequestInfo,
        _file_id: T,
        file_handle: FileHandle,
        _flags: OpenFlags,
        _lock_owner: Option<u64>,
        _flush: bool,
    ) -> FuseResult<()> {
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => posix_fs::release(fd),
            Err(e) => Err(e.into()),
        }
    }

    fn fsync(
        &self,
        _req: &RequestInfo,
        _file_id: T,
        file_handle: FileHandle,
        datasync: bool,
    ) -> FuseResult<()> {
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => posix_fs::fsync(&fd, datasync),
            Err(e) => Err(e.into()),
        }
    }

    fn fallocate(
        &self,
        _req: &RequestInfo,
        _file_id: T,
        file_handle: FileHandle,
        offset: i64,
        length: i64,
        mode: i32,
    ) -> FuseResult<()> {
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => posix_fs::fallocate(&fd, offset, length, mode),
            Err(e) => Err(e.into()),
        }
    }

    fn lseek(
        &self,
        _req: &RequestInfo,
        _file_id: T,
        file_handle: FileHandle,
        offset: i64,
        whence: Whence,
    ) -> FuseResult<i64> {
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => posix_fs::lseek(&fd, offset, whence),
            Err(e) => Err(e.into()),
        }
    }

    fn copy_file_range(
        &self,
        _req: &RequestInfo,
        _file_in: T,
        file_handle_in: FileHandle,
        offset_in: i64,
        _file_out: T,
        file_handle_out: FileHandle,
        offset_out: i64,
        len: u64,
        _flags: u32, // Not implemented yet in standard
    ) -> FuseResult<u32> {
        match (
            FileDescriptor::try_from(file_handle_in),
            FileDescriptor::try_from(file_handle_out),
        ) {
            (Ok(fd_in), Ok(fd_out)) => {
                posix_fs::copy_file_range(&fd_in, offset_in, &fd_out, offset_out, len)
            }
            (Err(e), _) | (_, Err(e)) => Err(e.into()),
        }
    }
}
