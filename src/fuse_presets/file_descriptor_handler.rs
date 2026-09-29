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
//! functions for use with `delegate_fs_async!`. On Linux, enabling `io_uring`
//! makes their `read`, `write`, `flush`, `fsync`, and `fallocate` methods use
//! io_uring. Other methods, and all methods on BSD/macOS, retain the synchronous
//! `unix_fs` implementation.
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

#[doc(hidden)]
#[macro_export]
macro_rules! file_descriptor_io_call {
    ($this:expr, flush, $fd:expr, async) => {{
        #[cfg(all(target_os = "linux", feature = "io_uring"))]
        {
            $this.io_uring.flush($fd).await
        }
        #[cfg(not(all(target_os = "linux", feature = "io_uring")))]
        {
            $crate::unix_fs::flush($fd)
        }
    }};
    ($this:expr, flush, $fd:expr) => {
        $crate::unix_fs::flush($fd)
    };
    ($this:expr, fsync, $fd:expr, $datasync:expr, async) => {{
        #[cfg(all(target_os = "linux", feature = "io_uring"))]
        {
            $this.io_uring.fsync($fd, $datasync).await
        }
        #[cfg(not(all(target_os = "linux", feature = "io_uring")))]
        {
            $crate::unix_fs::fsync($fd, $datasync)
        }
    }};
    ($this:expr, fsync, $fd:expr, $datasync:expr) => {
        $crate::unix_fs::fsync($fd, $datasync)
    };
    ($this:expr, release, $fd:expr, async) => {{
        let fd = $fd;
        #[cfg(all(target_os = "linux", feature = "io_uring"))]
        {
            let raw_fd = std::os::fd::AsRawFd::as_raw_fd(&fd);
            $this.io_uring.wait_for_fd(raw_fd).await;
            $crate::unix_fs::release(fd)
        }
        #[cfg(not(all(target_os = "linux", feature = "io_uring")))]
        {
            $crate::unix_fs::release(fd)
        }
    }};
    ($this:expr, release, $fd:expr) => {
        $crate::unix_fs::release($fd)
    };
    ($this:expr, read, $fd:expr, $seek:expr, $size:expr, async) => {{
        #[cfg(all(target_os = "linux", feature = "io_uring"))]
        {
            $this.io_uring.read($fd, $seek, $size).await
        }
        #[cfg(not(all(target_os = "linux", feature = "io_uring")))]
        {
            $crate::unix_fs::read($fd, $seek, $size)
        }
    }};
    ($this:expr, read, $fd:expr, $seek:expr, $size:expr) => {
        $crate::unix_fs::read($fd, $seek, $size)
    };
    ($this:expr, write, $fd:expr, $seek:expr, $data:expr, async) => {{
        let data = $data;
        #[cfg(all(target_os = "linux", feature = "io_uring"))]
        {
            $this.io_uring.write($fd, $seek, data).await
        }
        #[cfg(not(all(target_os = "linux", feature = "io_uring")))]
        {
            $crate::unix_fs::write($fd, $seek, &data)
        }
    }};
    ($this:expr, write, $fd:expr, $seek:expr, $data:expr) => {
        $crate::unix_fs::write($fd, $seek, &$data)
    };
    ($this:expr, fallocate, $fd:expr, $offset:expr, $length:expr, $mode:expr, async) => {{
        #[cfg(all(target_os = "linux", feature = "io_uring"))]
        {
            $this.io_uring.fallocate($fd, $offset, $length, $mode).await
        }
        #[cfg(not(all(target_os = "linux", feature = "io_uring")))]
        {
            $crate::unix_fs::fallocate($fd, $offset, $length, $mode)
        }
    }};
    ($this:expr, fallocate, $fd:expr, $offset:expr, $length:expr, $mode:expr) => {
        $crate::unix_fs::fallocate($fd, $offset, $length, $mode)
    };
}

macro_rules! file_descriptor_handler_readonly_methods {
    ($file_id:path $(, $asyncness:ident)?) => {
        pub $( $asyncness )? fn flush<'a>(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: Option<&mut OwnedFd>,
            _lock_owner: u64,
        ) -> FuseResult<()> {
            let fd = file_handle
                .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing file handle"))?;
            $crate::file_descriptor_io_call!(self, flush, fd.as_fd() $(, $asyncness)?)
        }

        pub $( $asyncness )? fn fsync<'a>(
            &self,
            _req: &RequestInfo,
            _file_id: $file_id,
            file_handle: Option<&mut OwnedFd>,
            datasync: bool,
        ) -> FuseResult<()> {
            let fd = file_handle
                .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing file handle"))?;
            $crate::file_descriptor_io_call!(self, fsync, fd.as_fd(), datasync $(, $asyncness)?)
        }

        pub $( $asyncness )? fn lseek<'a>(
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

        pub $( $asyncness )? fn read<'a>(
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
            $crate::file_descriptor_io_call!(self, read, fd.as_fd(), seek, size as usize $(, $asyncness)?)
        }

        pub $( $asyncness )? fn release(
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
            $crate::file_descriptor_io_call!(self, release, fd $(, $asyncness)?)
        }

        pub $( $asyncness )? fn getlk<'a>(
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

        pub $( $asyncness )? fn ioctl<'a>(
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

        pub $( $asyncness )? fn setlk<'a>(
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
    ($file_id:path $(, $asyncness:ident)?) => {
        pub $( $asyncness )? fn copy_file_range<'a, 'b>(
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

        pub $( $asyncness )? fn fallocate<'a>(
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
            $crate::file_descriptor_io_call!(self, fallocate, fd.as_fd(), offset, length, mode $(, $asyncness)?)
        }

        pub $( $asyncness )? fn write<'a>(
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
            $crate::file_descriptor_io_call!(self, write, fd.as_fd(), seek, data $(, $asyncness)?)
                .map(|res| res as u32)
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

/// Async file-descriptor helpers.
///
/// These methods satisfy the async `FuseHandler` interface. On Linux, enabling
/// the `io_uring` feature uses io_uring for `read`, `write`, `flush`, `fsync`,
/// and `fallocate`; remaining methods use synchronous `unix_fs` calls. On
/// BSD/macOS, methods use the synchronous implementation. The ring-backed
/// methods own their buffers while requests are in flight, and `release` waits
/// for requests submitted through this helper before closing the descriptor.
#[cfg(feature = "async")]
pub struct FileDescriptorHandlerAsync<TId: FileIdType> {
    phantom: PhantomData<TId>,
    #[cfg(all(target_os = "linux", feature = "io_uring"))]
    io_uring: unix_fs::io_uring::IoUringExecutor,
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
            #[cfg(all(target_os = "linux", feature = "io_uring"))]
            io_uring: unix_fs::io_uring::IoUringExecutor::new(),
        }
    }
}

#[cfg(feature = "async")]
impl<TId: FileIdType> FileDescriptorHandlerAsync<TId> {
    file_descriptor_handler_readonly_methods!(TId, async);
    file_descriptor_handler_readwrite_methods!(TId, async);
}

/// Async read-only file-descriptor helpers.
///
/// With Linux feature `io_uring`, `read`, `flush`, and `fsync` use io_uring;
/// other methods use synchronous `unix_fs` calls. On BSD/macOS they all remain
/// synchronous.
#[cfg(feature = "async")]
pub struct FileDescriptorHandlerReadOnlyAsync<TId: FileIdType> {
    phantom: PhantomData<TId>,
    #[cfg(all(target_os = "linux", feature = "io_uring"))]
    io_uring: unix_fs::io_uring::IoUringExecutor,
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
            #[cfg(all(target_os = "linux", feature = "io_uring"))]
            io_uring: unix_fs::io_uring::IoUringExecutor::new(),
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
