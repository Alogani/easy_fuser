use crate::types::*;
use crate::*;

use fuse_api::ReplyCb;
use templates::BaseFuse;


/// Implement all functions that rely on file handle to be done by assuming a file handle represents a file descriptor on the filesystem.
///
/// This consideration should be taken into account when deriving this trait in the implementation of the following functions:
/// - `open`
/// - `create`

pub struct FileDescriptorBridge {
    sublayer: BaseFuse
}

impl FileDescriptorBridge {
    pub fn new() -> Self {
        Self { sublayer: BaseFuse::new() }
    }
}

impl FuseAPI for FileDescriptorBridge {
    fn get_sublayer(&self) -> &impl FuseAPI {
        &self.sublayer
    }

    fn getattr(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: Option<FileHandle>,
        callback: ReplyCb<AttributeResponse>,
    ) {
        let fh = file_handle.expect("getattr requires a file_handle");
        match FileDescriptor::try_from(fh) {
            Ok(fd) => callback(posix_fs::getattr(&fd, Some(ino))),
            Err(e) => callback(Err(e.into())),
        }
    }

    fn read(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        offset: i64,
        size: u32,
        flags: FUSEReadFlags,
        lock_owner: Option<u64>,
        callback: ReplyCb<Vec<u8>>,
    ) {
        #![allow(unused_variables)]
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => callback(posix_fs::read(&fd, offset, size)),
            Err(e) => callback(Err(e.into())),
        }
    }

    fn write(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        offset: i64,
        data: &[u8],
        write_flags: FUSEWriteFlags,
        flags: OpenFlags,
        lock_owner: Option<u64>,
        callback: ReplyCb<u32>,
    ) {
        #![allow(unused_variables)]
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => callback(posix_fs::write(&fd, offset, data)),
            Err(e) => callback(Err(e.into())),
        }
    }

    fn flush(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        lock_owner: u64,
        callback: ReplyCb<()>,
    ) {
        #![allow(unused_variables)]
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => callback(posix_fs::flush(&fd)),
            Err(e) => callback(Err(e.into())),
        }
    }

    fn fsync(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        datasync: bool,
        callback: ReplyCb<()>,
    ) {
        #![allow(unused_variables)]
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => callback(posix_fs::fsync(&fd, datasync)),
            Err(e) => callback(Err(e.into())),
        }
    }

    fn release(
        &self,
        _req: RequestInfo,
        _ino: u64,
        _file_handle: FileHandle,
        _flags: OpenFlags,
        _lock_owner: Option<u64>,
        _flush: bool,
        callback: ReplyCb<()>,
    ) {
        match FileDescriptor::try_from(_file_handle) {
            Ok(fd) => callback(posix_fs::release(fd)),
            Err(e) => callback(Err(e.into())),
        }
    }

    fn fallocate(
        &self,
        _req: RequestInfo,
        ino: u64,
        _file_handle: FileHandle,
        offset: i64,
        length: i64,
        mode: i32,
        callback: ReplyCb<()>,
    ) {
        #![allow(unused_variables)]
        match FileDescriptor::try_from(_file_handle) {
            Ok(fd) => callback(posix_fs::fallocate(&fd, offset, length, mode)),
            Err(e) => callback(Err(e.into())),
        }
    }

    fn lseek(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        offset: i64,
        whence: Whence,
        callback: ReplyCb<i64>,
    ) {
        #![allow(unused_variables)]
        match FileDescriptor::try_from(file_handle) {
            Ok(fd) => callback(posix_fs::lseek(&fd, offset, whence)),
            Err(e) => callback(Err(e.into())),
        }
    }

    fn copy_file_range(
        &self,
        _req: RequestInfo,
        ino_in: u64,
        file_handle_in: FileHandle,
        offset_in: i64,
        ino_out: u64,
        file_handle_out: FileHandle,
        offset_out: i64,
        len: u64,
        _flags: u32, // Not implemented yet in standard
        callback: ReplyCb<u32>,
    ) {
        #![allow(unused_variables)]
        match (
            FileDescriptor::try_from(file_handle_in),
            FileDescriptor::try_from(file_handle_out),
        ) {
            (Ok(fd_in), Ok(fd_out)) => callback(posix_fs::copy_file_range(
                &fd_in, offset_in, &fd_out, offset_out, len,
            )),
            (Err(e), _) | (_, Err(e)) => callback(Err(e.into())),
        }
    }
}
