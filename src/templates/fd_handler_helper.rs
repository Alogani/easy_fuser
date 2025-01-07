/*!
# FdHandlerHelper and FdHandlerHelperReadOnly

Helper implementations for FUSE (Filesystem in Userspace) handlers that manage file operations using file descriptors.

## Overview

This module provides two helper structs:
1. `FdHandlerHelper<T>`: Implements the `FuseHandler<T>` trait for full read-write operations.
2. `FdHandlerHelperReadOnly<T>`: Implements the `FuseHandler<T>` trait for read-only operations.

Both helpers assume that file handles represent file descriptors on the filesystem.

## Implementation Details

### `FdHandlerHelper<T>`

Implements the following `FuseHandler<T>` methods:

- `read`: Reads data from a file using the file descriptor.
- `write`: Writes data to a file using the file descriptor.
- `flush`: Flushes the file associated with the file descriptor.
- `release`: Releases (closes) the file descriptor.
- `fsync`: Synchronizes the file's in-core state with storage device.
- `fallocate`: Manipulates the allocated disk space for the file.
- `lseek`: Repositions the file offset of the file descriptor.
- `copy_file_range`: Copies a range of data from one file to another.

### `FdHandlerHelperReadOnly<T>`

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

## Note
For more specific implementations or to extend functionality, you can modify these handlers or use them as a reference for implementing your own FuseHandler.

If you intend to enforce read-only at the fuse level,
prefer the usage of option `MountOption::RO` instead of `FdHandlerHelperReadOnly`.
*/

use crate::prelude::*;
use crate::unix_fs;

macro_rules! fd_handler_readonly_methods {
    () => {
        fn flush(
            &self,
            _req: &RequestInfo,
            _file_id: TId,
            file_handle: BorrowedFileHandle,
            _lock_owner: u64,
        ) -> FuseResult<()> {
            unix_fs::flush(file_handle.as_borrowed_fd())
        }

        fn fsync(
            &self,
            _req: &RequestInfo,
            _file_id: TId,
            file_handle: BorrowedFileHandle,
            datasync: bool,
        ) -> FuseResult<()> {
            unix_fs::fsync(file_handle.as_borrowed_fd(), datasync)
        }

        fn lseek(
            &self,
            _req: &RequestInfo,
            _file_id: TId,
            file_handle: BorrowedFileHandle,
            seek: SeekFrom,
        ) -> FuseResult<i64> {
            unix_fs::lseek(file_handle.as_borrowed_fd(), seek)
        }

        fn read(
            &self,
            _req: &RequestInfo,
            _file_id: TId,
            file_handle: BorrowedFileHandle,
            seek: SeekFrom,
            size: u32,
            _flags: FUSEOpenFlags,
            _lock_owner: Option<u64>,
        ) -> FuseResult<Vec<u8>> {
            unix_fs::read(file_handle.as_borrowed_fd(), seek, size as usize)
        }

        fn release(
            &self,
            _req: &RequestInfo,
            _file_id: TId,
            file_handle: OwnedFileHandle,
            _flags: OpenFlags,
            _lock_owner: Option<u64>,
            _flush: bool,
        ) -> FuseResult<()> {
            unix_fs::release(file_handle.into_owned_fd())
        }
    };
}

macro_rules! fd_handler_readwrite_methods {
    () => {
        fn copy_file_range(
            &self,
            _req: &RequestInfo,
            _file_in: TId,
            file_handle_in: BorrowedFileHandle,
            offset_in: i64,
            _file_out: TId,
            file_handle_out: BorrowedFileHandle,
            offset_out: i64,
            len: u64,
            _flags: u32,
        ) -> FuseResult<u32> {
            unix_fs::copy_file_range(
                file_handle_in.as_borrowed_fd(),
                offset_in,
                file_handle_out.as_borrowed_fd(),
                offset_out,
                len,
            )
        }

        fn fallocate(
            &self,
            _req: &RequestInfo,
            _file_id: TId,
            file_handle: BorrowedFileHandle,
            offset: i64,
            length: i64,
            mode: FallocateFlags,
        ) -> FuseResult<()> {
            unix_fs::fallocate(file_handle.as_borrowed_fd(), offset, length, mode)
        }

        fn write(
            &self,
            _req: &RequestInfo,
            _file_id: TId,
            file_handle: BorrowedFileHandle,
            seek: SeekFrom,
            data: Vec<u8>,
            _write_flags: FUSEWriteFlags,
            _flags: OpenFlags,
            _lock_owner: Option<u64>,
        ) -> FuseResult<u32> {
            unix_fs::write(file_handle.as_borrowed_fd(), seek, &data).map(|res| res as u32)
        }
    };
}

/// Specific documentation is located in parent module documentation.
pub struct FdHandlerHelper<TId: FileIdType> {
    inner: Box<dyn FuseHandler<TId>>,
}

impl<TId: FileIdType> FdHandlerHelper<TId> {
    pub fn new<THandler: FuseHandler<TId>>(inner: THandler) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
}

impl<TId: FileIdType> FuseHandler<TId> for FdHandlerHelper<TId> {
    fn get_inner(&self) -> &dyn FuseHandler<TId> {
        self.inner.as_ref()
    }

    fd_handler_readonly_methods!();
    fd_handler_readwrite_methods!();
}

/// Specific documentation is located in parent module documentation.
pub struct FdHandlerHelperReadOnly<TId: FileIdType> {
    inner: Box<dyn FuseHandler<TId>>,
}

impl<TId: FileIdType> FdHandlerHelperReadOnly<TId> {
    pub fn new<THandler: FuseHandler<TId>>(inner: THandler) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
}

impl<TId: FileIdType> FuseHandler<TId> for FdHandlerHelperReadOnly<TId> {
    fn get_inner(&self) -> &dyn FuseHandler<TId> {
        self.inner.as_ref()
    }

    fd_handler_readonly_methods!();
}
