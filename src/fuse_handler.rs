use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

use crate::types::*;

pub trait FuseHandler<T: FileIdType>: 'static {
    fn get_inner(&self) -> &Box<dyn FuseHandler<T>>;

    fn get_default_ttl(&self) -> Duration {
        Duration::from_secs(1)
    }

    fn init(&self, req: RequestInfo, config: &mut KernelConfig) -> FuseResult<()> {
        self.get_inner().init(req, config)
    }

    fn lookup(&self, req: RequestInfo, parent_id: T, name: &OsStr) -> FuseResult<FileAttribute> {
        self.get_inner().lookup(req, parent_id, name)
    }

    fn forget(&self, req: RequestInfo, file_id: T, nlookup: u64) {
        self.get_inner().forget(req, file_id, nlookup);
    }

    fn getattr(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: Option<FileHandle>,
    ) -> FuseResult<FileAttribute> {
        self.get_inner().getattr(req, file_id, file_handle)
    }

    fn setattr(
        &self,
        req: RequestInfo,
        file_id: T,
        attrs: SetAttrRequest,
    ) -> FuseResult<FileAttribute> {
        self.get_inner().setattr(req, file_id, attrs)
    }

    fn readlink(&self, req: RequestInfo, file_id: T) -> FuseResult<Vec<u8>> {
        self.get_inner().readlink(req, file_id)
    }

    fn mknod(
        &self,
        req: RequestInfo,
        parent_id: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: DeviceType,
    ) -> FuseResult<FileAttribute> {
        self.get_inner()
            .mknod(req, parent_id, name, mode, umask, rdev)
    }

    fn mkdir(
        &self,
        req: RequestInfo,
        parent_id: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
    ) -> FuseResult<FileAttribute> {
        self.get_inner().mkdir(req, parent_id, name, mode, umask)
    }

    fn unlink(&self, req: RequestInfo, parent_id: T, name: &OsStr) -> FuseResult<()> {
        self.get_inner().unlink(req, parent_id, name)
    }

    fn rmdir(&self, req: RequestInfo, parent_id: T, name: &OsStr) -> FuseResult<()> {
        self.get_inner().rmdir(req, parent_id, name)
    }

    fn symlink(
        &self,
        req: RequestInfo,
        parent_id: T,
        link_name: &OsStr,
        target: &Path,
    ) -> FuseResult<FileAttribute> {
        self.get_inner().symlink(req, parent_id, link_name, target)
    }

    fn rename(
        &self,
        req: RequestInfo,
        parent_id: T,
        name: &OsStr,
        newparent: T,
        newname: &OsStr,
        flags: RenameFlags,
    ) -> FuseResult<()> {
        self.get_inner()
            .rename(req, parent_id, name, newparent, newname, flags)
    }

    fn link(
        &self,
        req: RequestInfo,
        file_id: T,
        newparent: T,
        newname: &OsStr,
    ) -> FuseResult<FileAttribute> {
        self.get_inner().link(req, file_id, newparent, newname)
    }

    fn open(
        &self,
        req: RequestInfo,
        file_id: T,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        self.get_inner().open(req, file_id, flags)
    }

    fn read(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        offset: i64,
        size: u32,
        flags: FUSEReadFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        self.get_inner()
            .read(req, file_id, file_handle, offset, size, flags, lock_owner)
    }

    fn write(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        offset: i64,
        data: &[u8],
        write_flags: FUSEWriteFlags,
        flags: OpenFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<u32> {
        self.get_inner().write(
            req,
            file_id,
            file_handle,
            offset,
            data,
            write_flags,
            flags,
            lock_owner,
        )
    }

    fn flush(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        lock_owner: u64,
    ) -> FuseResult<()> {
        self.get_inner()
            .flush(req, file_id, file_handle, lock_owner)
    }

    fn fsync(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        datasync: bool,
    ) -> FuseResult<()> {
        self.get_inner().fsync(req, file_id, file_handle, datasync)
    }

    fn opendir(
        &self,
        req: RequestInfo,
        file_id: T,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        self.get_inner().opendir(req, file_id, flags)
    }

    fn readdir(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
    ) -> FuseResult<Vec<FuseDirEntry>> {
        self.get_inner().readdir(req, file_id, file_handle)
    }

    fn readdirplus(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
    ) -> FuseResult<Vec<FuseDirEntryPlus>> {
        self.get_inner().readdirplus(req, file_id, file_handle)
    }

    fn releasedir(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        flags: OpenFlags,
    ) -> FuseResult<()> {
        self.get_inner()
            .releasedir(req, file_id, file_handle, flags)
    }

    fn fsyncdir(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        datasync: bool,
    ) -> FuseResult<()> {
        self.get_inner()
            .fsyncdir(req, file_id, file_handle, datasync)
    }

    fn release(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        flags: OpenFlags,
        lock_owner: Option<u64>,
        flush: bool,
    ) -> FuseResult<()> {
        self.get_inner()
            .release(req, file_id, file_handle, flags, lock_owner, flush)
    }

    fn statfs(&self, req: RequestInfo, file_id: T) -> FuseResult<StatFs> {
        self.get_inner().statfs(req, file_id)
    }

    fn setxattr(
        &self,
        req: RequestInfo,
        file_id: T,
        name: &OsStr,
        value: &[u8],
        flags: FUSESetXAttrFlags,
        position: u32,
    ) -> FuseResult<()> {
        self.get_inner()
            .setxattr(req, file_id, name, value, flags, position)
    }

    fn getxattr(
        &self,
        req: RequestInfo,
        file_id: T,
        name: &OsStr,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        self.get_inner().getxattr(req, file_id, name, size)
    }

    fn listxattr(&self, req: RequestInfo, file_id: T, size: u32) -> FuseResult<Vec<u8>> {
        self.get_inner().listxattr(req, file_id, size)
    }

    fn removexattr(&self, req: RequestInfo, file_id: T, name: &OsStr) -> FuseResult<()> {
        self.get_inner().removexattr(req, file_id, name)
    }

    fn access(&self, req: RequestInfo, file_id: T, mask: AccessMask) -> FuseResult<()> {
        self.get_inner().access(req, file_id, mask)
    }

    fn getlk(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
    ) -> FuseResult<LockInfo> {
        self.get_inner()
            .getlk(req, file_id, file_handle, lock_owner, lock_info)
    }

    fn setlk(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
        sleep: bool,
    ) -> FuseResult<()> {
        self.get_inner()
            .setlk(req, file_id, file_handle, lock_owner, lock_info, sleep)
    }

    fn bmap(&self, req: RequestInfo, file_id: T, blocksize: u32, idx: u64) -> FuseResult<u64> {
        self.get_inner().bmap(req, file_id, blocksize, idx)
    }

    fn ioctl(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        flags: IOCtlFlags,
        cmd: u32,
        in_data: &[u8],
        out_size: u32,
    ) -> FuseResult<(i32, Vec<u8>)> {
        self.get_inner()
            .ioctl(req, file_id, file_handle, flags, cmd, in_data, out_size)
    }

    fn create(
        &self,
        req: RequestInfo,
        parent_id: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FileAttribute, FUSEOpenResponseFlags)> {
        self.get_inner()
            .create(req, parent_id, name, mode, umask, flags)
    }

    fn fallocate(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        offset: i64,
        length: i64,
        mode: i32,
    ) -> FuseResult<()> {
        self.get_inner()
            .fallocate(req, file_id, file_handle, offset, length, mode)
    }

    fn lseek(
        &self,
        req: RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        offset: i64,
        whence: Whence,
    ) -> FuseResult<i64> {
        self.get_inner()
            .lseek(req, file_id, file_handle, offset, whence)
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
        self.get_inner().copy_file_range(
            req,
            file_in,
            file_handle_in,
            offset_in,
            file_out,
            file_handle_out,
            offset_out,
            len,
            flags,
        )
    }
}
