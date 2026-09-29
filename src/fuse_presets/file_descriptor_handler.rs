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
use std::os::fd::{AsFd, OwnedFd};

macro_rules! file_descriptor_handler_readonly_methods {
    ($file_id:path) => {
        pub fn flush(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: Option<&mut OwnedFd>,
            _lock_owner: u64,
        ) -> FuseResult<()> {
            let fd = file_handle
                .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing file handle"))?;
            unix_fs::flush(fd.as_fd())
        }

        pub fn fsync(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: Option<&mut OwnedFd>,
            datasync: bool,
        ) -> FuseResult<()> {
            let fd = file_handle
                .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing file handle"))?;
            unix_fs::fsync(fd.as_fd(), datasync)
        }

        pub fn lseek(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: Option<&mut OwnedFd>,
            seek: SeekFrom,
        ) -> FuseResult<i64> {
            let fd = file_handle
                .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing file handle"))?;
            unix_fs::lseek(fd.as_fd(), seek)
        }

        pub fn read(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: Option<&mut OwnedFd>,
            seek: SeekFrom,
            size: u32,
            _flags: OpenFlags,
            _lock_owner: Option<u64>,
        ) -> FuseResult<Vec<u8>> {
            let fd = file_handle
                .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing file handle"))?;
            unix_fs::read(fd.as_fd(), seek, size as usize)
        }

        pub fn release(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: Option<OwnedFd>,
            _flags: OpenFlags,
            _lock_owner: Option<u64>,
            _flush: bool,
        ) -> FuseResult<()> {
            let fd = file_handle
                .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing file handle"))?;
            unix_fs::release(fd)
        }

        pub fn getlk(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: Option<&mut OwnedFd>,
            lock_owner: u64,
            lock_info: LockInfo,
        ) -> FuseResult<LockInfo> {
            let fd = file_handle
                .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing file handle"))?;
            unix_fs::getlk(fd.as_fd(), lock_owner, lock_info)
        }

        pub fn ioctl(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: Option<&mut OwnedFd>,
            _flags: IoctlFlags,
            cmd: u32,
            in_data: Vec<u8>,
            out_size: u32,
        ) -> FuseResult<(i32, Vec<u8>)> {
            let fd = file_handle
                .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing file handle"))?;
            unix_fs::ioctl(fd.as_fd(), cmd, in_data, out_size)
        }

        pub fn setlk(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: Option<&mut OwnedFd>,
            lock_owner: u64,
            lock_info: LockInfo,
            sleep: bool,
        ) -> FuseResult<()> {
            let fd = file_handle
                .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing file handle"))?;
            unix_fs::setlk(fd.as_fd(), lock_owner, lock_info, sleep)
        }
    };
}

macro_rules! file_descriptor_handler_readwrite_methods {
    ($file_id:path) => {
        pub fn copy_file_range(
            &self,
            _req: &RequestInfo,
            _file_in: $file_id,
            file_handle_in: Option<&OwnedFd>,
            offset_in: u64,
            _file_out: $file_id,
            file_handle_out: Option<&OwnedFd>,
            offset_out: u64,
            len: u64,
            _flags: CopyFileRangeFlags,
        ) -> FuseResult<u32> {
            unix_fs::copy_file_range(
                file_handle_in
                    .ok_or_else(|| {
                        ErrorKind::BadFileDescriptor.to_error("missing source file handle")
                    })?
                    .as_fd(),
                offset_in as i64,
                file_handle_out
                    .ok_or_else(|| {
                        ErrorKind::BadFileDescriptor.to_error("missing destination file handle")
                    })?
                    .as_fd(),
                offset_out as i64,
                len,
            )
        }

        pub fn fallocate(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: Option<&mut OwnedFd>,
            offset: i64,
            length: i64,
            mode: FallocateFlags,
        ) -> FuseResult<()> {
            let fd = file_handle
                .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing file handle"))?;
            unix_fs::fallocate(fd.as_fd(), offset, length, mode)
        }

        pub fn write(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: Option<&mut OwnedFd>,
            seek: SeekFrom,
            data: Vec<u8>,
            _write_flags: WriteFlags,
            _flags: OpenFlags,
            _lock_owner: Option<u64>,
        ) -> FuseResult<u32> {
            let fd = file_handle
                .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing file handle"))?;
            unix_fs::write(fd.as_fd(), seek, &data).map(|res| res as u32)
        }
    };
}

/// File-descriptor backed helpers for read and write operations.
///
/// Provides descriptor-backed `flush`, `fsync`, `getlk`, `ioctl`, `lseek`,
/// `read`, `release`, and `setlk`, plus `copy_file_range`, `fallocate`, and
/// `write`. Use `OwnedFd` as your handler's `FileHandle` and return it from
/// `open` and `create`.
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
/// descriptor as its `FileHandle`.
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

/// Deprecated alias for [`FileDescriptorHandler`].
#[deprecated(since = "0.8.0", note = "use `FileDescriptorHandler` instead")]
pub type FdHandlerHelper<TId> = FileDescriptorHandler<TId>;

/// Deprecated alias for [`FileDescriptorHandlerReadOnly`].
#[deprecated(since = "0.8.0", note = "use `FileDescriptorHandlerReadOnly` instead")]
pub type FdHandlerHelperReadOnly<TId> = FileDescriptorHandlerReadOnly<TId>;

pub(super) use file_descriptor_handler_readonly_methods;
pub(super) use file_descriptor_handler_readwrite_methods;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn missing_resource_returns_bad_file_descriptor() {
        let request = RequestInfo {
            id: RequestId(0),
            uid: 0,
            gid: 0,
            pid: 0,
        };
        let handler = FileDescriptorHandler::<PathBuf>::new();

        let read_error = handler
            .read(
                &request,
                PathBuf::new(),
                None,
                SeekFrom::Start(0),
                1,
                OpenFlags(0),
                None,
            )
            .unwrap_err();
        assert_eq!(read_error.kind(), ErrorKind::BadFileDescriptor);

        let release_error = handler
            .release(&request, PathBuf::new(), None, OpenFlags(0), None, false)
            .unwrap_err();
        assert_eq!(release_error.kind(), ErrorKind::BadFileDescriptor);
    }
}
