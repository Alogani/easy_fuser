use crate::posix_fs;
use crate::prelude::*;

/// Implement all functions that rely on file handle to be done by assuming a file handle represents a file descriptor on the filesystem.
///
/// This consideration should be taken into account when deriving this trait in the implementation of the following functions:
/// - `open`
/// - `create`

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
        eprintln!("FdBridge getinner");
        &self.inner
    }

    /*fn getattr(
        &self,
        _req: RequestInfo,
        _file_id: T,
        file_handle: Option<FileHandle>,
    ) -> FuseResult<FileAttribute> {
        let fh = file_handle.expect("getattr requires a file_handle");
        match FileDescriptor::try_from(fh) {
            Ok(fd) => posix_fs::getattr(&fd),
            Err(e) => Err(e.into()),
        }
    }*/

    fn read(
        &self,
        _req: RequestInfo,
        _file_id: T,
        file_handle: FileHandle,
        offset: i64,
        size: u32,
        _flags: FUSEReadFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => posix_fs::read(&fd, offset, size),
            Err(e) => Err(e.into()),
        }
    }

    fn write(
        &self,
        _req: RequestInfo,
        _file_id: T,
        file_handle: FileHandle,
        offset: i64,
        data: &[u8],
        _write_flags: FUSEWriteFlags,
        _flags: OpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<u32> {
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => posix_fs::write(&fd, offset, data),
            Err(e) => Err(e.into()),
        }
    }

    fn flush(
        &self,
        _req: RequestInfo,
        _file_id: T,
        file_handle: FileHandle,
        _lock_owner: u64,
    ) -> FuseResult<()> {
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => posix_fs::flush(&fd),
            Err(e) => Err(e.into()),
        }
    }

    fn fsync(
        &self,
        _req: RequestInfo,
        _file_id: T,
        file_handle: FileHandle,
        datasync: bool,
    ) -> FuseResult<()> {
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => posix_fs::fsync(&fd, datasync),
            Err(e) => Err(e.into()),
        }
    }

    fn release(
        &self,
        _req: RequestInfo,
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

    fn fallocate(
        &self,
        _req: RequestInfo,
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
        _req: RequestInfo,
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
        _req: RequestInfo,
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
