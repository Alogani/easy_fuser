use std::{ffi::OsStr, path::Path, time::Duration};

use fuser::KernelConfig;
use log::{debug, warn};

use crate::types::*;
use crate::wrapper::IdType;
use crate::FuseAPI;

/// Default skeleton, see templates to have ready to use fuse filesystem
///
/// The following functions are nevertheless implemented with a default response, so that implementing them is not needed
/// - `init` -> returns just an Ok response
/// - `opendir` -> returns a FileHandle with value 0
/// - `releasedir` -> returns just an Ok response
/// - `fsyncdir` -> returns just an Ok response
/// - `statsfs` -> return the value of StatFs::default


pub struct BaseFuse;

impl<T: IdType> FuseAPI<T> for BaseFuse
{
    #[allow(refining_impl_trait)]
    fn get_inner(&self) -> &BaseFuse {
        panic!("BaseFuse doesn't have inner API")
    }
    
    fn get_default_ttl() -> Duration {
        Duration::from_secs(1)
    }

    fn init(&self, _req: RequestInfo, _config: &mut KernelConfig) -> FuseResult<()> {
        Ok(())
    }

    fn lookup(&self, _req: RequestInfo, parent: T, name: &OsStr)
        -> FuseResult<FileAttribute> {
        warn!(
            "[Not Implemented] lookup(parent_file: {:?}, name {:?})",
            parent, name
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn forget(&self, _req: RequestInfo, _file: T, _nlookup: u64) {}

    fn getattr(
        &self,
        _req: RequestInfo,
        file: T,
        file_handle: Option<FileHandle>,
    ) -> FuseResult<FileAttribute> {
        warn!(
            "[Not Implemented] getattr(file: {:?}, file_handle {:?})",
            file, file_handle
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn setattr(
        &self,
        _req: RequestInfo,
        file: T,
        attrs: SetAttrRequest,
    ) -> FuseResult<FileAttribute> {
        debug!(
            "[Not Implemented] setattr(file: {:?}, _req: {:?}, attrs: {:?}",
            file, _req, attrs
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn readlink(&self, _req: RequestInfo, file: T) -> FuseResult<Vec<u8>> {
        debug!("[Not Implemented] readlink(file: {:?})", file);
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn mknod(
        &self,
        _req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: DeviceType,
    ) -> FuseResult<FileAttribute> {
        debug!(
            "[Not Implemented] mknod(parent: {:?}, name: {:?}, mode: {}, \
            umask: {:?}, rdev: {:?})",
            parent, name, mode, umask, rdev
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn mkdir(
        &self,
        _req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
    ) -> FuseResult<FileAttribute> {
        debug!(
            "[Not Implemented] mkdir(parent: {:?}, name: {:?}, mode: {}, umask: {:?})",
            parent, name, mode, umask
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn unlink(&self, _req: RequestInfo, parent: T, name: &OsStr) -> FuseResult<()> {
        debug!(
            "[Not Implemented] unlink(parent: {:?}, name: {:?})",
            parent, name,
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn rmdir(&self, _req: RequestInfo, parent: T, name: &OsStr) -> FuseResult<()> {
        debug!(
            "[Not Implemented] rmdir(parent: {:?}, name: {:?})",
            parent, name,
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn symlink(
        &self,
        _req: RequestInfo,
        parent: T,
        link_name: &OsStr,
        target: &Path,
    ) -> FuseResult<FileAttribute> {
        debug!(
            "[Not Implemented] symlink(parent: {:?}, link_name: {:?}, target: {:?})",
            parent, link_name, target,
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn rename(
        &self,
        _req: RequestInfo,
        parent: T,
        name: &OsStr,
        newparent: T,
        newname: &OsStr,
        flags: RenameFlags,
    ) -> FuseResult<()> {
        debug!(
            "[Not Implemented] rename(parent: {:?}, name: {:?}, newparent: {:?}, \
            newname: {:?}, flags: {:?})",
            parent, name, newparent, newname, flags,
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn link(
        &self,
        _req: RequestInfo,
        file: T,
        newparent: T,
        newname: &OsStr,
    ) -> FuseResult<FileAttribute> {
        debug!(
            "[Not Implemented] link(file: {:?}, newparent: {:?}, newname: {:?})",
            file, newparent, newname
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn open(
        &self,
        _req: RequestInfo,
        file: T,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        debug!(
            "[Not Implemented] open(file: {:?}, flags: {:?})",
            file, flags
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn read(
        &self,
        _req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        offset: i64,
        size: u32,
        flags: FUSEReadFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        debug!(
            "[Not Implemented] read(file: {:?}, file_handle: {:?}, offset: {}, size: {}, flags: {:?}, lock_owner: {:?})",
            file, file_handle, offset, size, flags, lock_owner
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn write(
        &self,
        _req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        offset: i64,
        data: &[u8],
        write_flags: FUSEWriteFlags,
        flags: OpenFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<u32> {
        debug!(
            "[Not Implemented] write(file: {:?}, file_handle: {:?}, offset: {}, data_len: {}, write_flags: {:?}, flags: {:?}, lock_owner: {:?})",
            file, file_handle, offset, data.len(), write_flags, flags, lock_owner
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn flush(
        &self,
        _req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        lock_owner: u64,
    ) -> FuseResult<()> {
        debug!(
            "[Not Implemented] flush(file: {:?}, file_handle: {:?}, lock_owner: {})",
            file, file_handle, lock_owner
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn fsync(
        &self,
        _req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        datasync: bool,
    ) -> FuseResult<()> {
        debug!(
            "[Not Implemented] fsync(file: {:?}, file_handle: {:?}, datasync: {})",
            file, file_handle, datasync
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn opendir(
        &self,
        _req: RequestInfo,
        _file: T,
        _flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        Ok((FileHandle::from(0), FUSEOpenResponseFlags::empty()))
    }

    fn readdir(
        &self,
        _req: RequestInfo,
        file: T,
        file_handle: FileHandle,
    ) -> FuseResult<Vec<FuseDirEntry>>{
        warn!(
            "[Not Implemented] readdir(file: {:?}, fh: {:?})",
            file, file_handle
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn readdirplus(
        &self,
        _req: RequestInfo,
        file: T,
        file_handle: FileHandle,
    ) -> FuseResult<Vec<FuseDirEntryPlus>> {
        warn!(
            "[Not Implemented] readdirplus(file: {:?}, fh: {:?})",
            file, file_handle
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn releasedir(
        &self,
        _req: RequestInfo,
        _file: T,
        _file_handle: FileHandle,
        _flags: OpenFlags,
    ) -> FuseResult<()> {
        Ok(())
    }

    fn fsyncdir(
        &self,
        _req: RequestInfo,
        _file: T,
        _file_handle: FileHandle,
        _datasync: bool,
    ) -> FuseResult<()> {
        Ok(())
    }

    fn release(
        &self,
        _req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        flags: OpenFlags,
        lock_owner: Option<u64>,
        flush: bool,
    ) -> FuseResult<()> {
        debug!(
            "[Not Implemented] release(file: {:?}, file_handle: {:?}, flags: {:?}, lock_owner: {:?}, flush: {:?})",
            file, file_handle, flags, lock_owner, flush
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn statfs(&self, _req: RequestInfo, _file: T) -> FuseResult<StatFs> {
        Ok(StatFs::default())
    }

    fn setxattr(
        &self,
        _req: RequestInfo,
        file: T,
        name: &OsStr,
        _value: &[u8],
        flags: FUSESetXAttrFlags,
        position: u32,
    ) -> FuseResult<()> {
        debug!(
            "[Not Implemented] setxattr(file: {:?}, name: {:?}, flags: {:?}, position: {})",
            file, name, flags, position
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn getxattr(
        &self,
        _req: RequestInfo,
        file: T,
        name: &OsStr,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        debug!(
            "[Not Implemented] getxattr(file: {:?}, name: {:?}, size: {})",
            file, name, size
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn listxattr(&self, _req: RequestInfo, file: T, size: u32) -> FuseResult<Vec<u8>> {
        debug!(
            "[Not Implemented] listxattr(file: {:?}, size: {})",
            file, size
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn removexattr(&self, _req: RequestInfo, file: T, name: &OsStr) -> FuseResult<()> {
        debug!(
            "[Not Implemented] removexattr(file: {:?}, name: {:?})",
            file, name
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn access(&self, _req: RequestInfo, file: T, mask: AccessMask) -> FuseResult<()> {
        debug!(
            "[Not Implemented] access(file: {:?}, mask: {:?})",
            file, mask
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn getlk(
        &self,
        _req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
    ) -> FuseResult<LockInfo> {
        debug!(
            "[Not Implemented] getlk(file: {:?}, fh: {:?}, lock_owner, {:?}, lock_info: {:?})",
            file, file_handle, lock_owner, lock_info
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn setlk(
        &self,
        _req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
        sleep: bool,
    ) -> FuseResult<()> {
        debug!(
            "[Not Implemented] setlk(file: {:?}, fh: {:?}, lock_owner, {:?}, lock_info: {:?}, sleep: {:?})",
            file, file_handle, lock_owner, lock_info, sleep
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn bmap(&self, _req: RequestInfo, file: T, blocksize: u32, idx: u64) -> FuseResult<u64> {
        debug!(
            "[Not Implemented] bmap(file: {:?}, blocksize: {:?}, idx: {:?})",
            file, blocksize, idx
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn ioctl(
        &self,
        _req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        flags: IOCtlFlags,
        cmd: u32,
        in_data: &[u8],
        out_size: u32,
    ) -> FuseResult<(i32, Vec<u8>)> {
        debug!(
            "[Not Implemented] ioctl(file: {:?}, fh: {:?}, flags: {:?}, cmd: {:?}, in_data: {:?}, out_size: {:?})",
            file, file_handle, flags, cmd, in_data, out_size
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn create(
        &self,
        _req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FileAttribute, FUSEOpenResponseFlags)> {
        debug!(
            "[Not Implemented] create(parent: {:?}, name: {:?}, mode: {}, umask: {:?}, \
            flags: {:?})",
            parent, name, mode, umask, flags
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn fallocate(
        &self,
        _req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        offset: i64,
        length: i64,
        mode: i32,
    ) -> FuseResult<()> {
        debug!(
            "[Not Implemented] fallocate(file: {:?}, file_handle: {:?} offset: {}, length: {}, mode: {})",
            file, file_handle, offset, length, mode
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn lseek(
        &self,
        _req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        offset: i64,
        whence: Whence,
    ) -> FuseResult<i64> {
        debug!(
            "[Not Implemented] lseek(file: {:?}, file_handle: {:?}, offset: {}, whence: {:?})",
            file, file_handle, offset, whence
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }

    fn copy_file_range(
        &self,
        _req: RequestInfo,
        file_in: T,
        file_handle_in: FileHandle,
        offset_in: i64,
        file_out: T,
        file_handle_out: FileHandle,
        offset_out: i64,
        len: u64,
        flags: u32, // Not implemented yet in standard
    ) -> FuseResult<u32> {
        debug!(
            "[Not Implemented] copy_file_range(file_in: {:?}, file_handle_in: {:?}, offset_in: {}, file_out: {:?}, file_handle_out: {:?}, offset_out: {}, len: {}, flags: {})",
            file_in, file_handle_in, offset_in, file_out, file_handle_out, offset_out, len, flags
        );
        Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
    }
}
