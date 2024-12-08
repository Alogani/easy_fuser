use std::{ffi::OsStr, io, path::Path, time::Duration};

use fuser::KernelConfig;
use log::{debug, warn};

use crate::{fuse_api::ReplyCb, types::*, FuseAPI};

/// Default skeleton, see templates to have ready to use fuse filesystem
///
/// The following functions are nevertheless implemented with a default response, so that implementing them is not needed
/// - `init` -> returns just an Ok response
/// - `opendir` -> returns a FileHandle with value 0
/// - `releasedir` -> returns just an Ok response
/// - `fsyncdir` -> returns just an Ok response
/// - `statsfs` -> return the value of StatFs::default

pub struct BaseFuse {}

impl BaseFuse {
    pub fn new() -> Self {
        BaseFuse { }
    }
}

impl FuseAPI for BaseFuse {
    #[allow(refining_impl_trait_reachable)]
    fn get_sublayer(&self) -> &BaseFuse {
        panic!("Base layer does not have a sublayer");
    }

    /// Function to get a default TTL of 1 second, that should be ok
    fn get_default_ttl(&self) -> Duration {
        Duration::from_secs(1)
    }

    fn init(&self, _req: RequestInfo, _config: &mut KernelConfig) -> Result<(), io::Error> {
        Ok(())
    }

    fn lookup(
        &self,
        _req: RequestInfo,
        parent_ino: u64,
        name: &OsStr,
        callback: ReplyCb<AttributeResponse>,
    ) {
        warn!(
            "[Not Implemented] lookup(parent_ino: {:#x?}, name {:?})",
            parent_ino, name
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn forget(&self, _req: RequestInfo, _ino: u64, _nlookup: u64) {}

    fn getattr(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: Option<FileHandle>,
        callback: ReplyCb<AttributeResponse>,
    ) {
        warn!(
            "[Not Implemented] getattr(ino: {:#x?}, file_handle {:?})",
            ino, file_handle
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn setattr(
        &self,
        _req: RequestInfo,
        ino: u64,
        attrs: SetAttrRequest,
        callback: ReplyCb<AttributeResponse>,
    ) {
        debug!(
            "[Not Implemented] setattr(ino: {:#x?}, req: {:?}, attrs: {:?}",
            ino, _req, attrs
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn readlink(
        &self,
        _req: RequestInfo,
        ino: u64,
        callback: Box<dyn FnOnce(Result<Vec<u8>, io::Error>) + Send>,
    ) {
        debug!("[Not Implemented] readlink(ino: {:#x?})", ino);
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn mknod(
        &self,
        _req: RequestInfo,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: DeviceType,
        callback: ReplyCb<AttributeResponse>,
    ) {
        debug!(
            "[Not Implemented] mknod(parent: {:#x?}, name: {:?}, mode: {}, \
            umask: {:#x?}, rdev: {:?})",
            parent, name, mode, umask, rdev
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn mkdir(
        &self,
        _req: RequestInfo,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        callback: ReplyCb<AttributeResponse>,
    ) {
        debug!(
            "[Not Implemented] mkdir(parent: {:#x?}, name: {:?}, mode: {}, umask: {:#x?})",
            parent, name, mode, umask
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn unlink(&self, _req: RequestInfo, parent: u64, name: &OsStr, callback: ReplyCb<()>) {
        debug!(
            "[Not Implemented] unlink(parent: {:#x?}, name: {:?})",
            parent, name,
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn rmdir(&self, _req: RequestInfo, parent: u64, name: &OsStr, callback: ReplyCb<()>) {
        debug!(
            "[Not Implemented] rmdir(parent: {:#x?}, name: {:?})",
            parent, name,
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn symlink(
        &self,
        _req: RequestInfo,
        parent: u64,
        link_name: &OsStr,
        target: &Path,
        callback: ReplyCb<AttributeResponse>,
    ) {
        debug!(
            "[Not Implemented] symlink(parent: {:#x?}, link_name: {:?}, target: {:?})",
            parent, link_name, target,
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn rename(
        &self,
        _req: RequestInfo,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        flags: RenameFlags,
        callback: ReplyCb<()>,
    ) {
        debug!(
            "[Not Implemented] rename(parent: {:#x?}, name: {:?}, newparent: {:#x?}, \
            newname: {:?}, flags: {:?})",
            parent, name, newparent, newname, flags,
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn link(
        &self,
        _req: RequestInfo,
        ino: u64,
        newparent: u64,
        newname: &OsStr,
        callback: ReplyCb<AttributeResponse>,
    ) {
        debug!(
            "[Not Implemented] link(ino: {:#x?}, newparent: {:#x?}, newname: {:?})",
            ino, newparent, newname
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn open(
        &self,
        _req: RequestInfo,
        _ino: u64,
        _flags: OpenFlags,
        callback: ReplyCb<(FileHandle, FUSEOpenResponseFlags)>,
    ) {
        debug!(
            "[Not Implemented] open(ino: {:#x?}, flags: {:?})",
            _ino, _flags
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
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
        debug!(
            "[Not Implemented] read(ino: {:#x}, file_handle: {:?}, offset: {}, size: {}, flags: {:?}, lock_owner: {:?})",
            ino, file_handle, offset, size, flags, lock_owner
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
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
        debug!(
            "[Not Implemented] write(ino: {:#x}, file_handle: {:?}, offset: {}, data_len: {}, write_flags: {:?}, flags: {:?}, lock_owner: {:?})",
            ino, file_handle, offset, data.len(), write_flags, flags, lock_owner
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn flush(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        lock_owner: u64,
        callback: ReplyCb<()>,
    ) {
        debug!(
            "[Not Implemented] flush(ino: {:#x}, file_handle: {:?}, lock_owner: {})",
            ino, file_handle, lock_owner
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn fsync(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        datasync: bool,
        callback: ReplyCb<()>,
    ) {
        debug!(
            "[Not Implemented] fsync(ino: {:#x}, file_handle: {:?}, datasync: {})",
            ino, file_handle, datasync
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn opendir(
        &self,
        _req: RequestInfo,
        _ino: u64,
        _flags: OpenFlags,
        callback: ReplyCb<(FileHandle, FUSEOpenResponseFlags)>,
    ) {
        callback(Ok((FileHandle::from(0), FUSEOpenResponseFlags::new())));
    }

    fn readdir(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        callback: ReplyCb<Vec<FuseDirEntry>>,
    ) {
        warn!(
            "[Not Implemented] readdir(ino: {:#x?}, fh: {:?})",
            ino, file_handle
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn readdirplus(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        callback: ReplyCb<Vec<FuseDirEntryPlus>>,
    ) {
        warn!(
            "[Not Implemented] readdirplus(ino: {:#x?}, fh: {:?})",
            ino, file_handle
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn releasedir(
        &self,
        _req: RequestInfo,
        _ino: u64,
        _file_handle: FileHandle,
        _flags: OpenFlags,
        callback: ReplyCb<()>,
    ) {
        callback(Ok(()))
    }

    fn fsyncdir(
        &self,
        _req: RequestInfo,
        _ino: u64,
        _file_handle: FileHandle,
        _datasync: bool,
        callback: ReplyCb<()>,
    ) {
        callback(Ok(()))
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
        debug!(
            "[Not Implemented] release(ino: {:#x}, file_handle: {:?}, flags: {:?}, lock_owner: {:?}, flush: {:?})",
            _ino, _file_handle, _flags, _lock_owner, _flush
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn statfs(&self, _req: RequestInfo, _ino: u64, callback: ReplyCb<StatFs>) {
        callback(Ok(StatFs::default()));
    }

    fn setxattr(
        &self,
        _req: RequestInfo,
        ino: u64,
        name: &OsStr,
        _value: &[u8],
        flags: FUSESetXAttrFlags,
        position: u32,
        callback: ReplyCb<()>,
    ) {
        debug!(
            "[Not Implemented] setxattr(ino: {:#x?}, name: {:?}, flags: {:#x?}, position: {})",
            ino, name, flags, position
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn getxattr(
        &self,
        _req: RequestInfo,
        ino: u64,
        name: &OsStr,
        size: u32,
        callback: ReplyCb<Vec<u8>>,
    ) {
        debug!(
            "[Not Implemented] getxattr(ino: {:#x?}, name: {:?}, size: {})",
            ino, name, size
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn listxattr(&self, _req: RequestInfo, ino: u64, size: u32, callback: ReplyCb<Vec<u8>>) {
        debug!(
            "[Not Implemented] listxattr(ino: {:#x?}, size: {})",
            ino, size
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn removexattr(&self, _req: RequestInfo, ino: u64, name: &OsStr, callback: ReplyCb<()>) {
        debug!(
            "[Not Implemented] removexattr(ino: {:#x?}, name: {:?})",
            ino, name
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn access(&self, _req: RequestInfo, ino: u64, mask: AccessMask, callback: ReplyCb<()>) {
        debug!(
            "[Not Implemented] access(ino: {:#x?}, mask: {:?})",
            ino, mask
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn getlk(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
        callback: ReplyCb<LockInfo>,
    ) {
        debug!(
            "[Not Implemented] getlk(ino: {:#x?}, fh: {:#x?}, lock_owner, {:?}, lock_info: {:?})",
            ino, file_handle, lock_owner, lock_info
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn setlk(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
        sleep: bool,
        callback: ReplyCb<()>,
    ) {
        debug!(
            "[Not Implemented] setlk(ino: {:#x?}, fh: {:#x?}, lock_owner, {:?}, lock_info: {:?}, sleep: {:#x?})",
            ino, file_handle, lock_owner, lock_info, sleep
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn bmap(&self, _req: RequestInfo, ino: u64, blocksize: u32, idx: u64, callback: ReplyCb<u64>) {
        debug!(
            "[Not Implemented] bmap(ino: {:#x?}, blocksize: {:#x?}, idx: {:#x?})",
            ino, blocksize, idx
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn ioctl(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        flags: IOCtlFlags,
        cmd: u32,
        in_data: &[u8],
        out_size: u32,
        callback: ReplyCb<(i32, Vec<u8>)>,
    ) {
        debug!(
            "[Not Implemented] ioctl(ino: {:#x?}, fh: {:#x?}, flags: {:#x?}, cmd: {:#x?}, in_data: {:?}, out_size: {:#x?})",
            ino, file_handle, flags, cmd, in_data, out_size
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }

    fn create(
        &self,
        _req: RequestInfo,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
        callback: ReplyCb<(FileHandle, AttributeResponse, FUSEOpenResponseFlags)>,
    ) {
        debug!(
            "[Not Implemented] create(parent: {:#x?}, name: {:?}, mode: {}, umask: {:#x?}, \
            flags: {:#x?})",
            parent, name, mode, umask, flags
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
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
        debug!(
            "[Not Implemented] fallocate(ino: {:#x}, offset: {}, length: {}, mode: {})",
            ino, offset, length, mode
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
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
        debug!(
            "[Not Implemented] lseek(ino: {:#x}, file_handle: {:?}, offset: {}, whence: {:?})",
            ino, file_handle, offset, whence
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
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
        debug!(
            "[Not Implemented] copy_file_range(ino_in: {:#x}, file_handle_in: {:?}, offset_in: {}, ino_out: {:#x}, file_handle_out: {:?}, offset_out: {}, len: {}, flags: {})",
            ino_in, file_handle_in, offset_in, ino_out, file_handle_out, offset_out, len, _flags
        );
        callback(Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into()));
    }
}
