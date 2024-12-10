use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::time::Duration;

use crate::types::*;
use crate::wrapper::IdType;

pub trait FuseAPI<T: IdType> {
    fn get_default_ttl() -> Duration {
        Duration::from_secs(1)
    }

    fn init(&self, req: RequestInfo, config: &mut KernelConfig) -> Result<(), io::Error>;

    fn lookup(&self, req: RequestInfo, parent: T, name: &OsStr)
        -> Result<FileAttribute, io::Error>;

    fn forget(&self, req: RequestInfo, file: T, nlookup: u64);

    fn getattr(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: Option<FileHandle>,
    ) -> Result<FileAttribute, io::Error>;

    fn setattr(
        &self,
        req: RequestInfo,
        file: T,
        attrs: SetAttrRequest,
    ) -> Result<FileAttribute, io::Error>;

    fn readlink(&self, req: RequestInfo, file: T) -> Result<Vec<u8>, io::Error>;

    fn mknod(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: DeviceType,
    ) -> Result<FileAttribute, io::Error>;

    fn mkdir(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
    ) -> Result<FileAttribute, io::Error>;

    fn unlink(&self, req: RequestInfo, parent: T, name: &OsStr) -> Result<(), io::Error>;

    fn rmdir(&self, req: RequestInfo, parent: T, name: &OsStr) -> Result<(), io::Error>;

    fn symlink(
        &self,
        req: RequestInfo,
        parent: T,
        link_name: &OsStr,
        target: &Path,
    ) -> Result<FileAttribute, io::Error>;

    fn rename(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        newparent: T,
        newname: &OsStr,
        flags: RenameFlags,
    ) -> Result<(), io::Error>;

    fn link(
        &self,
        req: RequestInfo,
        file: T,
        newparent: T,
        newname: &OsStr,
    ) -> Result<FileAttribute, io::Error>;

    fn open(
        &self,
        req: RequestInfo,
        file: T,
        flags: OpenFlags,
    ) -> Result<(FileHandle, FUSEOpenResponseFlags), io::Error>;

    fn read(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        offset: i64,
        size: u32,
        flags: FUSEReadFlags,
        lock_owner: Option<u64>,
    ) -> Result<Vec<u8>, io::Error>;

    fn write(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        offset: i64,
        data: &[u8],
        write_flags: FUSEWriteFlags,
        flags: OpenFlags,
        lock_owner: Option<u64>,
    ) -> Result<u32, io::Error>;

    fn flush(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        lock_owner: u64,
    ) -> Result<(), io::Error>;

    fn fsync(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        datasync: bool,
    ) -> Result<(), io::Error>;

    fn opendir(
        &self,
        req: RequestInfo,
        file: T,
        flags: OpenFlags,
    ) -> Result<(FileHandle, FUSEOpenResponseFlags), io::Error>;

    fn readdir(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
    ) -> Result<Vec<FuseDirEntry>, io::Error>;

    fn readdirplus(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
    ) -> Result<Vec<FuseDirEntryPlus>, io::Error>;

    fn releasedir(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        flags: OpenFlags,
    ) -> Result<(), io::Error>;

    fn fsyncdir(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        datasync: bool,
    ) -> Result<(), io::Error>;

    fn release(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        flags: OpenFlags,
        lock_owner: Option<u64>,
        flush: bool,
    ) -> Result<(), io::Error>;

    fn statfs(&self, req: RequestInfo, file: T) -> Result<StatFs, io::Error>;

    fn setxattr(
        &self,
        req: RequestInfo,
        file: T,
        name: &OsStr,
        value: &[u8],
        flags: FUSESetXAttrFlags,
        position: u32,
    ) -> Result<(), io::Error>;

    fn getxattr(
        &self,
        req: RequestInfo,
        file: T,
        name: &OsStr,
        size: u32,
    ) -> Result<Vec<u8>, io::Error>;

    fn listxattr(&self, req: RequestInfo, file: T, size: u32) -> Result<Vec<u8>, io::Error>;

    fn removexattr(&self, req: RequestInfo, file: T, name: &OsStr) -> Result<(), io::Error>;

    fn access(&self, req: RequestInfo, file: T, mask: AccessMask) -> Result<(), io::Error>;

    fn getlk(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
    ) -> Result<LockInfo, io::Error>;

    fn setlk(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
        sleep: bool,
    ) -> Result<(), io::Error>;

    fn bmap(&self, req: RequestInfo, file: T, blocksize: u32, idx: u64) -> Result<u64, io::Error>;

    fn ioctl(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        flags: IOCtlFlags,
        cmd: u32,
        in_data: &[u8],
        out_size: u32,
    ) -> Result<(i32, Vec<u8>), io::Error>;

    fn create(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
    ) -> Result<(FileHandle, FileAttribute, FUSEOpenResponseFlags), io::Error>;

    fn fallocate(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        offset: i64,
        length: i64,
        mode: i32,
    ) -> Result<(), io::Error>;

    fn lseek(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        offset: i64,
        whence: Whence,
    ) -> Result<i64, io::Error>;

    fn copy_file_range(
        &self,
        req: RequestInfo,
        file_in: T,
        file_handle_in: FileHandle,
        offset_in: i64,
        file_out: T,
        file_handle_out: FileHandle,
        offset_out: i64,
        len: u64,
        flags: u32, // Not implemented yet in standard
    ) -> Result<u32, io::Error>;
}
