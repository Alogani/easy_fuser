//! Read, write, and close files through file descriptors.
//!
//! These helpers provide selected FUSE methods for filesystems whose file
//! handles contain open file descriptors. They are method helpers, not complete
//! `FuseHandler` implementations. Add them as fields in your handler and
//! delegate only the methods they provide.
//!
//! # Methods provided
//!
//! `FileDescriptorHandlerReadOnly<TId>` provides `flush`, `fsync`, `getlk`,
//! `ioctl`, `lseek`, `read`, `release`, and `setlk`.
//!
//! `FileDescriptorHandler<TId>` provides those same methods, plus `copy_file_range`,
//! `fallocate`, and `write`.
//!
//! With the `async` feature, `FileDescriptorHandlerAsync` and
//! `FileDescriptorHandlerReadOnlyAsync` expose the same methods as async
//! functions for use with `delegate_fs_async!`. They call the synchronous
//! `unix_fs` functions directly; they do not offload blocking calls from the
//! async runtime.
//!
//! # Methods your filesystem still provides
//!
//! Your `open` and `create` methods must return valid open file descriptors as
//! file handles. The helpers use those descriptors for later reads or writes,
//! and `release` closes them. Other methods, such as `lookup`, `getattr`, and
//! `readdir`, belong to your filesystem or another preset.
//!
//! `FileDescriptorHandlerReadOnly` only omits write methods; use a read-only mount
//! option if the whole mounted filesystem must reject changes.

use crate::types::*;
use crate::unix_fs;
use std::marker::PhantomData;

macro_rules! file_descriptor_handler_readonly_methods {
    ($file_id:path $(, $asyncness:ident)?) => {
        pub $( $asyncness )? fn flush<'a>(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle<'a>,
            _lock_owner: u64,
        ) -> FuseResult<()> {
            unix_fs::flush(file_handle.as_borrowed_fd())
        }

        pub $( $asyncness )? fn fsync<'a>(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle<'a>,
            datasync: bool,
        ) -> FuseResult<()> {
            unix_fs::fsync(file_handle.as_borrowed_fd(), datasync)
        }

        pub $( $asyncness )? fn lseek<'a>(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle<'a>,
            seek: SeekFrom,
        ) -> FuseResult<i64> {
            unix_fs::lseek(file_handle.as_borrowed_fd(), seek)
        }

        pub $( $asyncness )? fn read<'a>(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle<'a>,
            seek: SeekFrom,
            size: u32,
            _flags: OpenFlags,
            _lock_owner: Option<u64>,
        ) -> FuseResult<Vec<u8>> {
            unix_fs::read(file_handle.as_borrowed_fd(), seek, size as usize)
        }

        pub $( $asyncness )? fn release(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: OwnedFileHandle,
            _flags: OpenFlags,
            _lock_owner: Option<u64>,
            _flush: bool,
        ) -> FuseResult<()> {
            unix_fs::release(file_handle.into_owned_fd())
        }

        pub $( $asyncness )? fn getlk<'a>(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle<'a>,
            lock_owner: u64,
            lock_info: LockInfo,
        ) -> FuseResult<LockInfo> {
            unix_fs::getlk(file_handle.as_borrowed_fd(), lock_owner, lock_info)
        }

        pub $( $asyncness )? fn ioctl<'a>(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle<'a>,
            _flags: IoctlFlags,
            cmd: u32,
            in_data: Vec<u8>,
            out_size: u32,
        ) -> FuseResult<(i32, Vec<u8>)> {
            unix_fs::ioctl(file_handle.as_borrowed_fd(), cmd, in_data, out_size)
        }

        pub $( $asyncness )? fn setlk<'a>(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle<'a>,
            lock_owner: u64,
            lock_info: LockInfo,
            sleep: bool,
        ) -> FuseResult<()> {
            unix_fs::setlk(file_handle.as_borrowed_fd(), lock_owner, lock_info, sleep)
        }
    };
}

macro_rules! file_descriptor_handler_readwrite_methods {
    ($file_id:path $(, $asyncness:ident)?) => {
        pub $( $asyncness )? fn copy_file_range<'a, 'b>(
            &self,
            _req: &RequestInfo,
            _file_in: $file_id,
            file_handle_in: BorrowedFileHandle<'a>,
            offset_in: u64,
            _file_out: $file_id,
            file_handle_out: BorrowedFileHandle<'b>,
            offset_out: u64,
            len: u64,
            _flags: CopyFileRangeFlags,
        ) -> FuseResult<u32> {
            unix_fs::copy_file_range(
                file_handle_in.as_borrowed_fd(),
                offset_in as i64,
                file_handle_out.as_borrowed_fd(),
                offset_out as i64,
                len,
            )
        }

        pub $( $asyncness )? fn fallocate<'a>(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle<'a>,
            offset: i64,
            length: i64,
            mode: FallocateFlags,
        ) -> FuseResult<()> {
            unix_fs::fallocate(file_handle.as_borrowed_fd(), offset, length, mode)
        }

        pub $( $asyncness )? fn write<'a>(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle<'a>,
            seek: SeekFrom,
            data: Vec<u8>,
            _write_flags: WriteFlags,
            _flags: OpenFlags,
            _lock_owner: Option<u64>,
        ) -> FuseResult<u32> {
            unix_fs::write(file_handle.as_borrowed_fd(), seek, &data).map(|res| res as u32)
        }
    };
}

/// File-descriptor backed helpers for read and write operations.
///
/// Provides descriptor-backed `flush`, `fsync`, `getlk`, `ioctl`, `lseek`,
/// `read`, `release`, and `setlk`, plus `copy_file_range`, `fallocate`, and
/// `write`. Your `open` and `create` methods must return an open file descriptor
/// as the file handle.
pub struct FileDescriptorHandler<TId: FileIdType> {
    phantom: PhantomData<TId>,
}

impl<TId: FileIdType> Default for FileDescriptorHandler<TId> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TId: FileIdType> FileDescriptorHandler<TId> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<TId: FileIdType> FileDescriptorHandler<TId> {
    file_descriptor_handler_readonly_methods!(TId);
    file_descriptor_handler_readwrite_methods!(TId);
}

/// Read-only file-descriptor backed helpers.
///
/// Provides descriptor-backed `flush`, `fsync`, `getlk`, `ioctl`, `lseek`,
/// `read`, `release`, and `setlk`. Your `open` method must return an open file
/// descriptor as the file handle.
pub struct FileDescriptorHandlerReadOnly<TId: FileIdType> {
    phantom: PhantomData<TId>,
}

impl<TId: FileIdType> Default for FileDescriptorHandlerReadOnly<TId> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TId: FileIdType> FileDescriptorHandlerReadOnly<TId> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<TId: FileIdType> FileDescriptorHandlerReadOnly<TId> {
    file_descriptor_handler_readonly_methods!(TId);
}

/// Async-compatible file-descriptor helpers.
///
/// These methods satisfy the async `FuseHandler` interface, but call the same
/// synchronous `unix_fs` operations directly. They do not move blocking system
/// calls off the Tokio runtime. Use them to compose an async handler with
/// `delegate_fs_async!`. Alternatively, use `delegate_fs_sync_to_async!` with
/// [`FileDescriptorHandler`]. A blocking-pool implementation may be added
/// separately if runtime responsiveness proves to require it.
#[cfg(feature = "async")]
pub struct FileDescriptorHandlerAsync<TId: FileIdType> {
    phantom: PhantomData<TId>,
}

#[cfg(feature = "async")]
impl<TId: FileIdType> Default for FileDescriptorHandlerAsync<TId> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "async")]
impl<TId: FileIdType> FileDescriptorHandlerAsync<TId> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

#[cfg(feature = "async")]
impl<TId: FileIdType> FileDescriptorHandlerAsync<TId> {
    file_descriptor_handler_readonly_methods!(TId, async);
    file_descriptor_handler_readwrite_methods!(TId, async);
}

/// Async-compatible read-only file-descriptor helpers.
///
/// These methods call synchronous `unix_fs` operations directly and do not
/// offload blocking system calls from the Tokio runtime.
#[cfg(feature = "async")]
pub struct FileDescriptorHandlerReadOnlyAsync<TId: FileIdType> {
    phantom: PhantomData<TId>,
}

#[cfg(feature = "async")]
impl<TId: FileIdType> Default for FileDescriptorHandlerReadOnlyAsync<TId> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "async")]
impl<TId: FileIdType> FileDescriptorHandlerReadOnlyAsync<TId> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

#[cfg(feature = "async")]
impl<TId: FileIdType> FileDescriptorHandlerReadOnlyAsync<TId> {
    file_descriptor_handler_readonly_methods!(TId, async);
}

/// Deprecated alias for [`FileDescriptorHandler`].
#[deprecated(since = "0.8.0", note = "use `FileDescriptorHandler` instead")]
pub type FdHandlerHelper<TId> = FileDescriptorHandler<TId>;

/// Deprecated alias for [`FileDescriptorHandlerReadOnly`].
#[deprecated(since = "0.8.0", note = "use `FileDescriptorHandlerReadOnly` instead")]
pub type FdHandlerHelperReadOnly<TId> = FileDescriptorHandlerReadOnly<TId>;

pub(super) use file_descriptor_handler_readonly_methods;
pub(super) use file_descriptor_handler_readwrite_methods;
