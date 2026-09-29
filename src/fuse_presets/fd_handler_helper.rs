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

macro_rules! fd_handler_readonly_methods {
    ($file_id:path) => {
        pub fn flush(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle,
            _lock_owner: u64,
        ) -> FuseResult<()> {
            unix_fs::flush(file_handle.as_borrowed_fd())
        }

        pub fn fsync(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle,
            datasync: bool,
        ) -> FuseResult<()> {
            unix_fs::fsync(file_handle.as_borrowed_fd(), datasync)
        }

        pub fn lseek(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle,
            seek: SeekFrom,
        ) -> FuseResult<i64> {
            unix_fs::lseek(file_handle.as_borrowed_fd(), seek)
        }

        pub fn read(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle,
            seek: SeekFrom,
            size: u32,
            _flags: OpenFlags,
            _lock_owner: Option<u64>,
        ) -> FuseResult<Vec<u8>> {
            unix_fs::read(file_handle.as_borrowed_fd(), seek, size as usize)
        }

        pub fn release(
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

        pub fn getlk(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle,
            lock_owner: u64,
            lock_info: LockInfo,
        ) -> FuseResult<LockInfo> {
            unix_fs::getlk(file_handle.as_borrowed_fd(), lock_owner, lock_info)
        }

        pub fn ioctl(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle,
            _flags: IoctlFlags,
            cmd: u32,
            in_data: Vec<u8>,
            out_size: u32,
        ) -> FuseResult<(i32, Vec<u8>)> {
            unix_fs::ioctl(file_handle.as_borrowed_fd(), cmd, in_data, out_size)
        }

        pub fn setlk(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle,
            lock_owner: u64,
            lock_info: LockInfo,
            sleep: bool,
        ) -> FuseResult<()> {
            unix_fs::setlk(file_handle.as_borrowed_fd(), lock_owner, lock_info, sleep)
        }
    };
}

macro_rules! fd_handler_readwrite_methods {
    ($file_id:path) => {
        pub fn copy_file_range(
            &self,
            _req: &RequestInfo,
            _file_in: $file_id,
            file_handle_in: BorrowedFileHandle,
            offset_in: u64,
            _file_out: $file_id,
            file_handle_out: BorrowedFileHandle,
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

        pub fn fallocate(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle,
            offset: i64,
            length: i64,
            mode: FallocateFlags,
        ) -> FuseResult<()> {
            unix_fs::fallocate(file_handle.as_borrowed_fd(), offset, length, mode)
        }

        pub fn write(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: BorrowedFileHandle,
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
    fd_handler_readonly_methods!(TId);
    fd_handler_readwrite_methods!(TId);
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
    fd_handler_readonly_methods!(TId);
}

/// Deprecated alias for [`FileDescriptorHandler`].
#[deprecated(since = "0.8.0", note = "use `FileDescriptorHandler` instead")]
pub type FdHandlerHelper<TId> = FileDescriptorHandler<TId>;

/// Deprecated alias for [`FileDescriptorHandlerReadOnly`].
#[deprecated(since = "0.8.0", note = "use `FileDescriptorHandlerReadOnly` instead")]
pub type FdHandlerHelperReadOnly<TId> = FileDescriptorHandlerReadOnly<TId>;

pub(super) use fd_handler_readonly_methods;
pub(super) use fd_handler_readwrite_methods;
