use std::{ffi::OsStr, io, path::Path, time::Duration};

use crate::wrapper::IdType;

use super::types::*;

pub type ReplyCb<T> = Box<dyn FnOnce(Result<T, io::Error>) + Send>;

pub trait FuseAPI<T: IdType> {
    fn get_sublayer(&self) -> &impl FuseAPI<T>;

    fn get_default_ttl(&self) -> Duration {
        self.get_sublayer().get_default_ttl()
    }

    fn init(&self, req: RequestInfo, config: &mut KernelConfig) -> Result<(), io::Error> {
        self.get_sublayer().init(req, config)
    }

    fn lookup(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        callback: ReplyCb<AttributeResponse>,
    ) {
        self.get_sublayer().lookup(req, parent, name, callback);
    }

    fn forget(&self, req: RequestInfo, file: T, nlookup: u64) {
        self.get_sublayer().forget(req, file, nlookup);
    }

    fn getattr(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: Option<FileHandle>,
        callback: ReplyCb<AttributeResponse>,
    ) {
        self.get_sublayer()
            .getattr(req, file, file_handle, callback);
    }

    fn setattr(
        &self,
        req: RequestInfo,
        file: T,
        attrs: SetAttrRequest,
        callback: ReplyCb<AttributeResponse>,
    ) {
        self.get_sublayer().setattr(req, file, attrs, callback);
    }

    fn readlink(
        &self,
        req: RequestInfo,
        file: T,
        callback: Box<dyn FnOnce(Result<Vec<u8>, io::Error>) + Send>,
    ) {
        self.get_sublayer().readlink(req, file, callback);
    }

    fn mknod(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: DeviceType,
        callback: ReplyCb<AttributeResponse>,
    ) {
        self.get_sublayer()
            .mknod(req, parent, name, mode, umask, rdev, callback);
    }

    fn mkdir(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        callback: ReplyCb<AttributeResponse>,
    ) {
        self.get_sublayer()
            .mkdir(req, parent, name, mode, umask, callback);
    }

    fn unlink(&self, req: RequestInfo, parent: T, name: &OsStr, callback: ReplyCb<()>) {
        self.get_sublayer().unlink(req, parent, name, callback);
    }

    fn rmdir(&self, req: RequestInfo, parent: T, name: &OsStr, callback: ReplyCb<()>) {
        self.get_sublayer().rmdir(req, parent, name, callback);
    }

    fn symlink(
        &self,
        req: RequestInfo,
        parent: T,
        link_name: &OsStr,
        target: &Path,
        callback: ReplyCb<AttributeResponse>,
    ) {
        self.get_sublayer()
            .symlink(req, parent, link_name, target, callback);
    }

    fn rename(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        newparent: T,
        newname: &OsStr,
        flags: RenameFlags,
        callback: ReplyCb<()>,
    ) {
        self.get_sublayer()
            .rename(req, parent, name, newparent, newname, flags, callback);
    }

    fn link(
        &self,
        req: RequestInfo,
        file: T,
        newparent: T,
        newname: &OsStr,
        callback: ReplyCb<AttributeResponse>,
    ) {
        self.get_sublayer()
            .link(req, file, newparent, newname, callback);
    }

    fn open(
        &self,
        req: RequestInfo,
        file: T,
        flags: OpenFlags,
        callback: ReplyCb<(FileHandle, FUSEOpenResponseFlags)>,
    ) {
        self.get_sublayer().open(req, file, flags, callback);
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
        callback: ReplyCb<Vec<u8>>,
    ) {
        self.get_sublayer().read(
            req,
            file,
            file_handle,
            offset,
            size,
            flags,
            lock_owner,
            callback,
        );
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
        callback: ReplyCb<u32>,
    ) {
        self.get_sublayer().write(
            req,
            file,
            file_handle,
            offset,
            data,
            write_flags,
            flags,
            lock_owner,
            callback,
        );
    }

    fn flush(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        lock_owner: u64,
        callback: ReplyCb<()>,
    ) {
        self.get_sublayer()
            .flush(req, file, file_handle, lock_owner, callback);
    }

    fn fsync(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        datasync: bool,
        callback: ReplyCb<()>,
    ) {
        self.get_sublayer()
            .fsync(req, file, file_handle, datasync, callback);
    }

    fn opendir(
        &self,
        req: RequestInfo,
        file: T,
        flags: OpenFlags,
        callback: ReplyCb<(FileHandle, FUSEOpenResponseFlags)>,
    ) {
        self.get_sublayer().opendir(req, file, flags, callback);
    }

    fn readdir(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        callback: ReplyCb<Vec<FuseDirEntry>>,
    ) {
        self.get_sublayer()
            .readdir(req, file, file_handle, callback);
    }

    fn readdirplus(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        callback: ReplyCb<Vec<FuseDirEntryPlus>>,
    ) {
        self.get_sublayer()
            .readdirplus(req, file, file_handle, callback);
    }

    fn releasedir(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        flags: OpenFlags,
        callback: ReplyCb<()>,
    ) {
        self.get_sublayer()
            .releasedir(req, file, file_handle, flags, callback);
    }

    fn fsyncdir(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        datasync: bool,
        callback: ReplyCb<()>,
    ) {
        self.get_sublayer()
            .fsyncdir(req, file, file_handle, datasync, callback);
    }

    fn release(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        flags: OpenFlags,
        lock_owner: Option<u64>,
        flush: bool,
        callback: ReplyCb<()>,
    ) {
        self.get_sublayer()
            .release(req, file, file_handle, flags, lock_owner, flush, callback);
    }

    fn statfs(&self, req: RequestInfo, file: T, callback: ReplyCb<StatFs>) {
        self.get_sublayer().statfs(req, file, callback);
    }

    fn setxattr(
        &self,
        req: RequestInfo,
        file: T,
        name: &OsStr,
        value: &[u8],
        flags: FUSESetXAttrFlags,
        position: u32,
        callback: ReplyCb<()>,
    ) {
        self.get_sublayer()
            .setxattr(req, file, name, value, flags, position, callback);
    }

    fn getxattr(
        &self,
        req: RequestInfo,
        file: T,
        name: &OsStr,
        size: u32,
        callback: ReplyCb<Vec<u8>>,
    ) {
        self.get_sublayer()
            .getxattr(req, file, name, size, callback);
    }

    fn listxattr(&self, req: RequestInfo, file: T, size: u32, callback: ReplyCb<Vec<u8>>) {
        self.get_sublayer().listxattr(req, file, size, callback);
    }

    fn removexattr(&self, req: RequestInfo, file: T, name: &OsStr, callback: ReplyCb<()>) {
        self.get_sublayer().removexattr(req, file, name, callback);
    }

    fn access(&self, req: RequestInfo, file: T, mask: AccessMask, callback: ReplyCb<()>) {
        self.get_sublayer().access(req, file, mask, callback);
    }

    fn getlk(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
        callback: ReplyCb<LockInfo>,
    ) {
        self.get_sublayer()
            .getlk(req, file, file_handle, lock_owner, lock_info, callback);
    }

    fn setlk(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
        sleep: bool,
        callback: ReplyCb<()>,
    ) {
        self.get_sublayer().setlk(
            req,
            file,
            file_handle,
            lock_owner,
            lock_info,
            sleep,
            callback,
        );
    }

    fn bmap(&self, req: RequestInfo, file: T, blocksize: u32, idx: u64, callback: ReplyCb<u64>) {
        self.get_sublayer()
            .bmap(req, file, blocksize, idx, callback);
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
        callback: ReplyCb<(i32, Vec<u8>)>,
    ) {
        self.get_sublayer().ioctl(
            req,
            file,
            file_handle,
            flags,
            cmd,
            in_data,
            out_size,
            callback,
        );
    }

    fn create(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
        callback: ReplyCb<(FileHandle, AttributeResponse, FUSEOpenResponseFlags)>,
    ) {
        self.get_sublayer()
            .create(req, parent, name, mode, umask, flags, callback);
    }

    fn fallocate(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        offset: i64,
        length: i64,
        mode: i32,
        callback: ReplyCb<()>,
    ) {
        self.get_sublayer()
            .fallocate(req, file, file_handle, offset, length, mode, callback);
    }

    fn lseek(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        offset: i64,
        whence: Whence,
        callback: ReplyCb<i64>,
    ) {
        self.get_sublayer()
            .lseek(req, file, file_handle, offset, whence, callback);
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
        callback: ReplyCb<u32>,
    ) {
        self.get_sublayer().copy_file_range(
            req,
            file_in,
            file_handle_in,
            offset_in,
            file_out,
            file_handle_out,
            offset_out,
            len,
            flags,
            callback,
        );
    }
}
