/*!
# FdHandlerHelper and FdHandlerHelperReadOnly

Helper implementations for FUSE (Filesystem in Userspace) handlers that manage file operations using file descriptors.

## Overview

This module provides two helper structs:
1. `FdHandlerHelper<T>`: Implements the `FuseHandler<T>` trait for full read-write operations.
2. `FdHandlerHelperReadOnly<T>`: Implements the `FuseHandler<T>` trait for read-only operations.

Both helpers assume that file handles represent file descriptors on the filesystem.

## Implementation Details

### FdHandlerHelper<T>

Implements the following `FuseHandler<T>` methods:

- `read`: Reads data from a file using the file descriptor.
- `write`: Writes data to a file using the file descriptor.
- `flush`: Flushes the file associated with the file descriptor.
- `release`: Releases (closes) the file descriptor.
- `fsync`: Synchronizes the file's in-core state with storage device.
- `fallocate`: Manipulates the allocated disk space for the file.
- `lseek`: Repositions the file offset of the file descriptor.
- `copy_file_range`: Copies a range of data from one file to another.

### FdHandlerHelperReadOnly<T>

Implements a subset of `FuseHandler<T>` methods for read-only operations:

- `read`: Reads data from a file using the file descriptor.
- `flush`: Flushes the file associated with the file descriptor.
- `release`: Releases (closes) the file descriptor.
- `fsync`: Synchronizes the file's in-core state with storage device.
- `lseek`: Repositions the file offset of the file descriptor.

## Usage

To use these helpers:

1. Create an instance of `FdHandlerHelper<T>` or `FdHandlerHelperReadOnly<T>` by passing an inner `FuseHandler<T>` implementation.
2. Use it as delegator in your own FUSE filesystem implementation (see FuseHandler documentation for more details).

## Important Considerations

When implementing the `open` and `create` methods in your filesystem:

- Ensure that the returned file handle can be converted to a valid file descriptor.
- The file handle should represent an open file descriptor on the underlying filesystem.

## Example

```text
let inner_handler = YourInnerHandler::new(); // or DefaultFuseHandler::new{};
let fd_handler = FdHandlerHelper::new(inner_handler);
// Use fd_handler as your primary FuseHandler

// For read-only operations:
let read_only_handler = FdHandlerHelperReadOnly::new(inner_handler); // or DefaultFuseHandler::new{};
// Use read_only_handler as your primary FuseHandler for read-only operations
```
*/

use std::marker::PhantomData;

use crate::posix_fs;
use crate::prelude::*;

macro_rules! fd_handler_readonly_methods {
    () => {
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
    };
}

macro_rules! fd_handler_readwrite_methods {
    () => {
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
            _flags: u32,
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
    };
}

/// Specific documentation is located in parent module documentation.
pub struct FdHandlerHelper<T: FileIdType, U: FuseHandler<T>> {
    phantom: PhantomData<T>,
    inner: U,
}

impl<T, U> FdHandlerHelper<T, U>
where
    T: FileIdType,
    U: FuseHandler<T>,
{
    pub fn new(inner: U) -> Self {
        Self {
            phantom: PhantomData,
            inner: inner,
        }
    }
}

impl<T, U> FuseHandler<T> for FdHandlerHelper<T, U>
where 
    T: FileIdType,
    U: FuseHandler<T>,
{
    fn get_inner(&self) -> &dyn FuseHandler<T> {
        &self.inner
    }

    fd_handler_readonly_methods!();
    fd_handler_readwrite_methods!();
}

/// Specific documentation is located in parent module documentation.
pub struct FdHandlerHelperReadOnly<T: FileIdType, U: FuseHandler<T>> {
    phantom: PhantomData<T>,
    inner: U,
}

impl<T, U> FdHandlerHelperReadOnly<T, U>
where
    T: FileIdType,
    U: FuseHandler<T>,
{
    pub fn new(inner: U) -> Self {
        Self {
            phantom: PhantomData,
            inner: inner,
        }
    }
}

impl<T, U> FuseHandler<T> for FdHandlerHelperReadOnly<T, U>
where 
    T: FileIdType,
    U: FuseHandler<T>,
{
    fn get_inner(&self) -> &dyn FuseHandler<T> {
        &self.inner
    }

    fd_handler_readonly_methods!();
}