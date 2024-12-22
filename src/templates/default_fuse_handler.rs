use std::{ffi::OsStr, path::Path, time::Duration};

use fuser::KernelConfig;

use crate::prelude::*;

/// Default skeleton, see templates to have ready to use fuse filesystem
///
/// The following functions are nevertheless implemented with a default response, so that implementing them is not needed
/// - `init` -> returns just an Ok response
/// - `opendir` -> returns a FileHandle with value 0
/// - `releasedir` -> returns just an Ok response
/// - `fsyncdir` -> returns just an Ok response
/// - `statsfs` -> return the value of StatFs::default

pub struct DefaultFuseHandler {
    panic: bool,
}

impl DefaultFuseHandler {
    pub fn new() -> Self {
        DefaultFuseHandler { panic: false }
    }

    pub fn new_with_panic() -> Self {
        DefaultFuseHandler { panic: true }
    }
}

impl<T: FileIdType> FuseHandler<T> for DefaultFuseHandler {
    fn get_inner(&self) -> &Box<(dyn FuseHandler<T>)> {
        panic!("Base Fuse don't have inner type")
    }

    fn get_default_ttl(&self) -> Duration {
        Duration::from_secs(1)
    }

    fn init(&self, _req: RequestInfo, _config: &mut KernelConfig) -> FuseResult<()> {
        Ok(())
    }

    fn destroy(&self) {}

    fn lookup(&self, _req: RequestInfo, parent_id: T, name: &OsStr) -> FuseResult<FileAttribute> {
        let msg = format!(
            "[Not Implemented] lookup(parent_file: {}, name {:?})",
            parent_id.display(),
            Path::display(name.as_ref())
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn forget(&self, _req: RequestInfo, _file_id: T, _nlookup: u64) {}

    fn getattr(
        &self,
        _req: RequestInfo,
        file_id: T,
        file_handle: Option<FileHandle>,
    ) -> FuseResult<FileAttribute> {
        let msg = format!(
            "[Not Implemented] getattr(file_id: {}, file_handle {:?})",
            file_id.display(),
            file_handle
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn setattr(
        &self,
        _req: RequestInfo,
        file_id: T,
        attrs: SetAttrRequest,
    ) -> FuseResult<FileAttribute> {
        let msg = format!(
            "[Not Implemented] setattr(file_id: {}, _req: {:?}, attrs: {:?}",
            file_id.display(),
            _req,
            attrs
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn readlink(&self, _req: RequestInfo, file_id: T) -> FuseResult<Vec<u8>> {
        let msg = format!("[Not Implemented] readlink(file_id: {})", file_id.display());
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn mknod(
        &self,
        _req: RequestInfo,
        parent_id: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: DeviceType,
    ) -> FuseResult<FileAttribute> {
        let msg = format!(
            "[Not Implemented] mknod(parent_id: {:?}, name: {:?}, mode: {}, \
            umask: {:?}, rdev: {:?})",
            parent_id, name, mode, umask, rdev
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn mkdir(
        &self,
        _req: RequestInfo,
        parent_id: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
    ) -> FuseResult<FileAttribute> {
        let msg = format!(
            "[Not Implemented] mkdir(parent_id: {:?}, name: {:?}, mode: {}, umask: {:?})",
            parent_id, name, mode, umask
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn unlink(&self, _req: RequestInfo, parent_id: T, name: &OsStr) -> FuseResult<()> {
        let msg = format!(
            "[Not Implemented] unlink(parent_id: {:?}, name: {:?})",
            parent_id, name,
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn rmdir(&self, _req: RequestInfo, parent_id: T, name: &OsStr) -> FuseResult<()> {
        let msg = format!(
            "[Not Implemented] rmdir(parent_id: {:?}, name: {:?})",
            parent_id, name,
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn symlink(
        &self,
        _req: RequestInfo,
        parent_id: T,
        link_name: &OsStr,
        target: &Path,
    ) -> FuseResult<FileAttribute> {
        let msg = format!(
            "[Not Implemented] symlink(parent_id: {:?}, link_name: {:?}, target: {:?})",
            parent_id, link_name, target,
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn rename(
        &self,
        _req: RequestInfo,
        parent_id: T,
        name: &OsStr,
        newparent: T,
        newname: &OsStr,
        flags: RenameFlags,
    ) -> FuseResult<()> {
        let msg = format!(
            "[Not Implemented] rename(parent_id: {:?}, name: {:?}, newparent: {:?}, \
            newname: {:?}, flags: {:?})",
            parent_id, name, newparent, newname, flags,
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn link(
        &self,
        _req: RequestInfo,
        file_id: T,
        newparent: T,
        newname: &OsStr,
    ) -> FuseResult<FileAttribute> {
        let msg = format!(
            "[Not Implemented] link(file_id: {}, newparent: {:?}, newname: {:?})",
            file_id.display(),
            newparent,
            newname
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn open(
        &self,
        _req: RequestInfo,
        file_id: T,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        let msg = format!(
            "[Not Implemented] open(file_id: {}, flags: {:?})",
            file_id.display(),
            flags
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn read(
        &self,
        _req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        offset: i64,
        size: u32,
        flags: FUSEOpenFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        let msg = format!(
            "[Not Implemented] read(file_id: {}, file_handle: {:?}, offset: {}, size: {}, flags: {:?}, lock_owner: {:?})",
            file_id.display(), file_handle, offset, size, flags, lock_owner
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn write(
        &self,
        _req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        offset: i64,
        data: &[u8],
        write_flags: FUSEWriteFlags,
        flags: OpenFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<u32> {
        let msg = format!(
            "[Not Implemented] write(file_id: {}, file_handle: {:?}, offset: {}, data_len: {}, write_flags: {:?}, flags: {:?}, lock_owner: {:?})",
            file_id.display(), file_handle, offset, data.len(), write_flags, flags, lock_owner
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn flush(
        &self,
        _req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        lock_owner: u64,
    ) -> FuseResult<()> {
        let msg = format!(
            "[Not Implemented] flush(file_id: {}, file_handle: {:?}, lock_owner: {})",
            file_id.display(),
            file_handle,
            lock_owner
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn release(
        &self,
        _req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        flags: OpenFlags,
        lock_owner: Option<u64>,
        flush: bool,
    ) -> FuseResult<()> {
        let msg = format!(
            "[Not Implemented] release(file_id: {}, file_handle: {:?}, flags: {:?}, lock_owner: {:?}, flush: {:?})",
            file_id.display(), file_handle, flags, lock_owner, flush
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn fsync(
        &self,
        _req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        datasync: bool,
    ) -> FuseResult<()> {
        let msg = format!(
            "[Not Implemented] fsync(file_id: {}, file_handle: {:?}, datasync: {})",
            file_id.display(),
            file_handle,
            datasync
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn opendir(
        &self,
        _req: RequestInfo,
        _file_id: T,
        _flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        Ok((FileHandle::from(0), FUSEOpenResponseFlags::empty()))
    }

    fn readdir(
        &self,
        _req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
    ) -> FuseResult<Vec<FuseDirEntry>> {
        let msg = format!(
            "[Not Implemented] readdir(file_id: {}, fh: {:?})",
            file_id.display(),
            file_handle
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn readdirplus(
        &self,
        _req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
    ) -> FuseResult<Vec<FuseDirEntryPlus>> {
        let msg = format!(
            "[Not Implemented] readdirplus(file_id: {}, fh: {:?})",
            file_id.display(),
            file_handle
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn releasedir(
        &self,
        _req: RequestInfo,
        _file_id: T,
        _file_handle: FileHandle,
        _flags: OpenFlags,
    ) -> FuseResult<()> {
        Ok(())
    }

    fn fsyncdir(
        &self,
        _req: RequestInfo,
        _file_id: T,
        _file_handle: FileHandle,
        _datasync: bool,
    ) -> FuseResult<()> {
        Ok(())
    }

    fn statfs(&self, _req: RequestInfo, _file_id: T) -> FuseResult<StatFs> {
        Ok(StatFs::default())
    }

    fn setxattr(
        &self,
        _req: RequestInfo,
        file_id: T,
        name: &OsStr,
        _value: &[u8],
        flags: FUSESetXAttrFlags,
        position: u32,
    ) -> FuseResult<()> {
        let msg = format!(
            "[Not Implemented] setxattr(file_id: {}, name: {:?}, flags: {:?}, position: {})",
            file_id.display(),
            name,
            flags,
            position
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn getxattr(
        &self,
        _req: RequestInfo,
        file_id: T,
        name: &OsStr,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        let msg = format!(
            "[Not Implemented] getxattr(file_id: {}, name: {:?}, size: {})",
            file_id.display(),
            name,
            size
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn listxattr(&self, _req: RequestInfo, file_id: T, size: u32) -> FuseResult<Vec<u8>> {
        let msg = format!(
            "[Not Implemented] listxattr(file_id: {}, size: {})",
            file_id.display(),
            size
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn removexattr(&self, _req: RequestInfo, file_id: T, name: &OsStr) -> FuseResult<()> {
        let msg = format!(
            "[Not Implemented] removexattr(file_id: {}, name: {:?})",
            file_id.display(),
            name
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn access(&self, _req: RequestInfo, file_id: T, mask: AccessMask) -> FuseResult<()> {
        let msg = format!(
            "[Not Implemented] access(file_id: {}, mask: {:?})",
            file_id.display(),
            mask
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn getlk(
        &self,
        _req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
    ) -> FuseResult<LockInfo> {
        let msg = format!(
            "[Not Implemented] getlk(file_id: {}, fh: {:?}, lock_owner, {:?}, lock_info: {:?})",
            file_id.display(),
            file_handle,
            lock_owner,
            lock_info
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn setlk(
        &self,
        _req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
        sleep: bool,
    ) -> FuseResult<()> {
        let msg = format!(
            "[Not Implemented] setlk(file_id: {}, fh: {:?}, lock_owner, {:?}, lock_info: {:?}, sleep: {:?})",
            file_id.display(), file_handle, lock_owner, lock_info, sleep
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn bmap(&self, _req: RequestInfo, file_id: T, blocksize: u32, idx: u64) -> FuseResult<u64> {
        let msg = format!(
            "[Not Implemented] bmap(file_id: {}, blocksize: {:?}, idx: {:?})",
            file_id.display(),
            blocksize,
            idx
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn ioctl(
        &self,
        _req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        flags: IOCtlFlags,
        cmd: u32,
        in_data: &[u8],
        out_size: u32,
    ) -> FuseResult<(i32, Vec<u8>)> {
        let msg = format!(
            "[Not Implemented] ioctl(file_id: {}, fh: {:?}, flags: {:?}, cmd: {:?}, in_data: {:?}, out_size: {:?})",
            file_id.display(), file_handle, flags, cmd, in_data, out_size
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn create(
        &self,
        _req: RequestInfo,
        parent_id: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FileAttribute, FUSEOpenResponseFlags)> {
        let msg = format!(
            "[Not Implemented] create(parent_id: {:?}, name: {:?}, mode: {}, umask: {:?}, \
            flags: {:?})",
            parent_id, name, mode, umask, flags
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn fallocate(
        &self,
        _req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        offset: i64,
        length: i64,
        mode: i32,
    ) -> FuseResult<()> {
        let msg = format!(
            "[Not Implemented] fallocate(file_id: {}, file_handle: {:?} offset: {}, length: {}, mode: {})",
            file_id.display(), file_handle, offset, length, mode
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }

    fn lseek(
        &self,
        _req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        offset: i64,
        whence: Whence,
    ) -> FuseResult<i64> {
        let msg = format!(
            "[Not Implemented] lseek(file_id: {}, file_handle: {:?}, offset: {}, whence: {:?})",
            file_id.display(),
            file_handle,
            offset,
            whence
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
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
        let msg = format!(
            "[Not Implemented] copy_file_range(file_in: {:?}, file_handle_in: {:?}, offset_in: {}, file_out: {:?}, file_handle_out: {:?}, offset_out: {}, len: {}, flags: {})",
            file_in, file_handle_in, offset_in, file_out, file_handle_out, offset_out, len, flags
        );
        if self.panic {
            panic!("{}", msg);
        } else {
            Err(PosixError::new(ErrorKind::FunctionNotImplemented, msg))
        }
    }
}
