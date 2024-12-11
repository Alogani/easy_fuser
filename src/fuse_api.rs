use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

use crate::types::*;
use crate::wrapper::IdType;

pub trait FuseAPI<T: IdType> {
    fn get_inner(&self) -> &impl FuseAPI<T>;

    fn get_default_ttl() -> Duration {
        Duration::from_secs(1)
    }

    fn init(&self, req: RequestInfo, config: &mut KernelConfig) -> FuseResult<()> {
        self.get_inner().init(req, config)
    }

    fn lookup(&self, req: RequestInfo, parent: T, name: &OsStr)
        -> FuseResult<FileAttribute> {
            self.get_inner().lookup(req, parent, name)
        }

    fn forget(&self, req: RequestInfo, file: T, nlookup: u64) {
        self.get_inner().forget(req, file, nlookup);
    }

    fn getattr(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: Option<FileHandle>,
    ) -> FuseResult<FileAttribute> {
        self.get_inner().getattr(req, file, file_handle)
    }

    fn setattr(
        &self,
        req: RequestInfo,
        file: T,
        attrs: SetAttrRequest,
    ) -> FuseResult<FileAttribute> {
        self.get_inner().setattr(req, file, attrs)
    }

    fn readlink(&self, req: RequestInfo, file: T) -> FuseResult<Vec<u8>> {
        self.get_inner().readlink(req, file)
    }

    fn mknod(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: DeviceType,
    ) -> FuseResult<FileAttribute> {
        self.get_inner().mknod(req, parent, name, mode, umask, rdev)
    }

    fn mkdir(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
    ) -> FuseResult<FileAttribute> {
        self.get_inner().mkdir(req, parent, name, mode, umask)
    }

    fn unlink(&self, req: RequestInfo, parent: T, name: &OsStr) -> FuseResult<()> {
        self.get_inner().unlink(req, parent, name)
    }

    fn rmdir(&self, req: RequestInfo, parent: T, name: &OsStr) -> FuseResult<()> {
        self.get_inner().rmdir(req, parent, name)
    }

    fn symlink(
        &self,
        req: RequestInfo,
        parent: T,
        link_name: &OsStr,
        target: &Path,
    ) -> FuseResult<FileAttribute> {
        self.get_inner().symlink(req, parent, link_name, target)
    }

    fn rename(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        newparent: T,
        newname: &OsStr,
        flags: RenameFlags,
    ) -> FuseResult<()> {
        self.get_inner().rename(req, parent, name, newparent, newname, flags)
    }

    fn link(
        &self,
        req: RequestInfo,
        file: T,
        newparent: T,
        newname: &OsStr,
    ) -> FuseResult<FileAttribute> {
        self.get_inner().link(req, file, newparent, newname)
    }

    fn open(
        &self,
        req: RequestInfo,
        file: T,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        self.get_inner().open(req, file, flags)
    }

    fn read(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        offset: i64,
        size: u32,
        flags: FUSEReadFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        self.get_inner().read(req, file, file_handle, offset, size, flags, lock_owner)
    }

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
    ) -> FuseResult<u32> {
        self.get_inner().write(req, file, file_handle, offset, data, write_flags, flags, lock_owner)
    }

    fn flush(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        lock_owner: u64,
    ) -> FuseResult<()> {
        self.get_inner().flush(req, file, file_handle, lock_owner)
    }

    fn fsync(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        datasync: bool,
    ) -> FuseResult<()> {
        self.get_inner().fsync(req, file, file_handle, datasync)
    }

    fn opendir(
        &self,
        req: RequestInfo,
        file: T,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        self.get_inner().opendir(req, file, flags)
    }

    fn readdir(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
    ) -> FuseResult<Vec<FuseDirEntry>> {
        self.get_inner().readdir(req, file, file_handle)
    }

    fn readdirplus(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
    ) -> FuseResult<Vec<FuseDirEntryPlus>> {
        self.get_inner().readdirplus(req, file, file_handle)
    }

    fn releasedir(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        flags: OpenFlags,
    ) -> FuseResult<()> {
        self.get_inner().releasedir(req, file, file_handle, flags)
    }

    fn fsyncdir(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        datasync: bool,
    ) -> FuseResult<()> {
        self.get_inner().fsync(req, file, file_handle, datasync)
    }

    fn release(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        flags: OpenFlags,
        lock_owner: Option<u64>,
        flush: bool,
    ) -> FuseResult<()> {
        self.get_inner().release(req, file, file_handle, flags, lock_owner, flush)
    }

    fn statfs(&self, req: RequestInfo, file: T) -> FuseResult<StatFs> {
        self.get_inner().statfs(req, file)
    }

    fn setxattr(
        &self,
        req: RequestInfo,
        file: T,
        name: &OsStr,
        value: &[u8],
        flags: FUSESetXAttrFlags,
        position: u32,
    ) -> FuseResult<()> {
        self.get_inner().setxattr(req, file, name, value, flags, position)
    }

    fn getxattr(
        &self,
        req: RequestInfo,
        file: T,
        name: &OsStr,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        self.get_inner().getxattr(req, file, name, size)
    }

    fn listxattr(&self, req: RequestInfo, file: T, size: u32) -> FuseResult<Vec<u8>> {
        self.get_inner().listxattr(req, file, size)
    }

    fn removexattr(&self, req: RequestInfo, file: T, name: &OsStr) -> FuseResult<()> {
        self.get_inner().removexattr(req, file, name)
    }

    fn access(&self, req: RequestInfo, file: T, mask: AccessMask) -> FuseResult<()> {
        self.get_inner().access(req, file, mask)
    }

    fn getlk(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
    ) -> FuseResult<LockInfo> {
        self.get_inner().getlk(req, file, file_handle, lock_owner, lock_info)
    }

    fn setlk(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
        sleep: bool,
    ) -> FuseResult<()> {
        self.get_inner().setlk(req, file, file_handle, lock_owner, lock_info, sleep)
    }

    fn bmap(&self, req: RequestInfo, file: T, blocksize: u32, idx: u64) -> FuseResult<u64> {
        self.get_inner().bmap(req, file, blocksize, idx)
    }

    fn ioctl(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        flags: IOCtlFlags,
        cmd: u32,
        in_data: &[u8],
        out_size: u32,
    ) -> FuseResult<(i32, Vec<u8>)> {
        self.get_inner().ioctl(req, file, file_handle, flags, cmd, in_data, out_size)
    }

    fn create(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FileAttribute, FUSEOpenResponseFlags)> {
        self.get_inner().create(req, parent, name, mode, umask, flags)
    }

    fn fallocate(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        offset: i64,
        length: i64,
        mode: i32,
    ) -> FuseResult<()> {
        self.get_inner().fallocate(req, file, file_handle, offset, length, mode)
    }

    fn lseek(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        offset: i64,
        whence: Whence,
    ) -> FuseResult<i64> {
        self.get_inner().lseek(req, file, file_handle, offset, whence)
    }

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
    ) -> FuseResult<u32> {
        self.get_inner().copy_file_range(req, file_in, file_handle_in, offset_in, file_out, file_handle_out, offset_out, len, flags)
    }
}
