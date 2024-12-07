use crate::types::*;
use crate::*;
use std::io;

/// Implement all functions that rely on file handle to be done by assuming a file handle represents a file descriptor on the filesystem.
///
/// This consideration should be taken into account when deriving this trait in the implementation of the following functions:
/// - `open`
/// - `create`
///
todo!() // REFACTOR
pub trait FileDescriptorBridge: FuseAPI {
    fn getattr(
        &mut self,
        _req: RequestInfo,
        ino: u64,
        file_handle: Option<FileHandle>,
    ) -> Result<AttributeResponse, io::Error> {
        if let Some(fh) = file_handle {
            posix_fs::getattr(&fh.try_into()?, Some(ino))
        } else {
            panic!("getattr current implementation won't work without a file_handle")
        }
    }

    fn read(
        &mut self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        offset: i64,
        size: u32,
        flags: FUSEReadFlags,
        lock_owner: Option<u64>,
    ) -> Result<Vec<u8>, io::Error> {
        #![allow(unused_variables)]
        posix_fs::read(&file_handle.try_into()?, offset, size)
    }

    fn write(
        &mut self,
        _req: RequestInfo,
        inode: u64,
        file_handle: FileHandle,
        offset: i64,
        data: &[u8],
        write_flags: FUSEWriteFlags,
        flags: OpenFlags,
        lock_owner: Option<u64>,
    ) -> Result<u32, io::Error> {
        #![allow(unused_variables)]
        posix_fs::write(&file_handle.try_into()?, offset, data)
    }

    fn flush(
        &mut self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        lock_owner: u64,
    ) -> Result<(), io::Error> {
        #![allow(unused_variables)]
        posix_fs::flush(&file_handle.try_into()?)
    }

    fn fsync(
        &mut self,
        _req: RequestInfo,
        inode: u64,
        file_handle: FileHandle,
        datasync: bool,
    ) -> Result<(), io::Error> {
        #![allow(unused_variables)]
        posix_fs::fsync(&file_handle.try_into()?, datasync)
    }

    fn release(
        &mut self,
        _req: RequestInfo,
        _ino: u64,
        _file_handle: FileHandle,
        _flags: OpenFlags,
        _lock_owner: Option<u64>,
        _flush: bool,
    ) -> Result<(), io::Error> {
        posix_fs::release(_file_handle.try_into()?)
    }

    fn fallocate(
        &mut self,
        _req: RequestInfo,
        ino: u64,
        _file_handle: FileHandle,
        offset: i64,
        length: i64,
        mode: i32,
    ) -> Result<(), io::Error> {
        #![allow(unused_variables)]
        posix_fs::fallocate(&_file_handle.try_into()?, offset, length, mode)
    }

    fn lseek(
        &mut self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        offset: i64,
        whence: Whence,
    ) -> Result<i64, io::Error> {
        #![allow(unused_variables)]
        posix_fs::lseek(&file_handle.try_into()?, offset, whence)
    }

    fn copy_file_range(
        &mut self,
        _req: RequestInfo,
        ino_in: u64,
        file_handle_in: FileHandle,
        offset_in: i64,
        ino_out: u64,
        file_handle_out: FileHandle,
        offset_out: i64,
        len: u64,
        flags: u32, // Not implemented yet in standard
    ) -> Result<u32, io::Error> {
        #![allow(unused_variables)]
        posix_fs::copy_file_range(
            &file_handle_in.try_into()?,
            offset_in,
            &file_handle_out.try_into()?,
            offset_out,
            len,
        )
    }
}
