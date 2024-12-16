use std::{ffi::OsStr, io, path::Path, time::Duration};

use crate::fuse_handler::FuseHandler;
use crate::types::*;

pub type ReplyCb<T> = Box<dyn FnOnce(Result<T, io::Error>) + Send>;

pub trait FuseCallbackHandler<T: FileIdType> {
    fn get_default_ttl() -> Duration {
        Duration::from_secs(1)
    }

    fn get_fuse_handler(&self) -> &impl FuseHandler<T>;

    fn init(&self, req: RequestInfo, config: &mut KernelConfig) -> Result<(), io::Error> {
        self.get_fuse_handler().init(req, config)
    }

    fn lookup(&self, req: RequestInfo, parent: T, name: &OsStr, callback: ReplyCb<FileAttribute>) {
        callback(self.get_fuse_handler().lookup(req, parent, name));
    }

    fn forget(&self, req: RequestInfo, file: T, nlookup: u64) {
        self.get_fuse_handler().forget(req, file, nlookup);
    }

    fn getattr(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: Option<FileHandle>,
        callback: ReplyCb<FileAttribute>,
    ) {
        callback(self.get_fuse_handler().getattr(req, file, file_handle));
    }

    fn setattr(
        &self,
        req: RequestInfo,
        file: T,
        attrs: SetAttrRequest,
        callback: ReplyCb<FileAttribute>,
    ) {
        callback(self.get_fuse_handler().setattr(req, file, attrs));
    }

    fn readlink(
        &self,
        req: RequestInfo,
        file: T,
        callback: Box<dyn FnOnce(Result<Vec<u8>, io::Error>) + Send>,
    ) {
        callback(self.get_fuse_handler().readlink(req, file));
    }

    fn mknod(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: DeviceType,
        callback: ReplyCb<FileAttribute>,
    ) {
        callback(
            self.get_fuse_handler()
                .mknod(req, parent, name, mode, umask, rdev),
        );
    }

    fn mkdir(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        callback: ReplyCb<FileAttribute>,
    ) {
        callback(
            self.get_fuse_handler()
                .mkdir(req, parent, name, mode, umask),
        );
    }

    fn unlink(&self, req: RequestInfo, parent: T, name: &OsStr, callback: ReplyCb<()>) {
        callback(self.get_fuse_handler().unlink(req, parent, name));
    }

    fn rmdir(&self, req: RequestInfo, parent: T, name: &OsStr, callback: ReplyCb<()>) {
        callback(self.get_fuse_handler().rmdir(req, parent, name));
    }

    fn symlink(
        &self,
        req: RequestInfo,
        parent: T,
        link_name: &OsStr,
        target: &Path,
        callback: ReplyCb<FileAttribute>,
    ) {
        callback(
            self.get_fuse_handler()
                .symlink(req, parent, link_name, target),
        );
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
        callback(
            self.get_fuse_handler()
                .rename(req, parent, name, newparent, newname, flags),
        );
    }

    fn link(
        &self,
        req: RequestInfo,
        file: T,
        newparent: T,
        newname: &OsStr,
        callback: ReplyCb<FileAttribute>,
    ) {
        callback(self.get_fuse_handler().link(req, file, newparent, newname));
    }

    fn open(
        &self,
        req: RequestInfo,
        file: T,
        flags: OpenFlags,
        callback: ReplyCb<(FileHandle, FUSEOpenResponseFlags)>,
    ) {
        callback(self.get_fuse_handler().open(req, file, flags));
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
        callback(self.get_fuse_handler().read(
            req,
            file,
            file_handle,
            offset,
            size,
            flags,
            lock_owner,
        ));
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
        callback(self.get_fuse_handler().write(
            req,
            file,
            file_handle,
            offset,
            data,
            write_flags,
            flags,
            lock_owner,
        ));
    }

    fn flush(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        lock_owner: u64,
        callback: ReplyCb<()>,
    ) {
        callback(
            self.get_fuse_handler()
                .flush(req, file, file_handle, lock_owner),
        );
    }

    fn fsync(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        datasync: bool,
        callback: ReplyCb<()>,
    ) {
        callback(
            self.get_fuse_handler()
                .fsync(req, file, file_handle, datasync),
        );
    }

    fn opendir(
        &self,
        req: RequestInfo,
        file: T,
        flags: OpenFlags,
        callback: ReplyCb<(FileHandle, FUSEOpenResponseFlags)>,
    ) {
        callback(self.get_fuse_handler().opendir(req, file, flags));
    }

    fn readdir(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        callback: ReplyCb<Vec<FuseDirEntry>>,
    ) {
        callback(self.get_fuse_handler().readdir(req, file, file_handle));
    }

    fn readdirplus(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        callback: ReplyCb<Vec<FuseDirEntryPlus>>,
    ) {
        callback(self.get_fuse_handler().readdirplus(req, file, file_handle));
    }

    fn releasedir(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        flags: OpenFlags,
        callback: ReplyCb<()>,
    ) {
        callback(
            self.get_fuse_handler()
                .releasedir(req, file, file_handle, flags),
        );
    }

    fn fsyncdir(
        &self,
        req: RequestInfo,
        file: T,
        file_handle: FileHandle,
        datasync: bool,
        callback: ReplyCb<()>,
    ) {
        callback(
            self.get_fuse_handler()
                .fsyncdir(req, file, file_handle, datasync),
        );
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
        callback(
            self.get_fuse_handler()
                .release(req, file, file_handle, flags, lock_owner, flush),
        );
    }

    fn statfs(&self, req: RequestInfo, file: T, callback: ReplyCb<StatFs>) {
        callback(self.get_fuse_handler().statfs(req, file));
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
        callback(
            self.get_fuse_handler()
                .setxattr(req, file, name, value, flags, position),
        );
    }

    fn getxattr(
        &self,
        req: RequestInfo,
        file: T,
        name: &OsStr,
        size: u32,
        callback: ReplyCb<Vec<u8>>,
    ) {
        callback(self.get_fuse_handler().getxattr(req, file, name, size));
    }

    fn listxattr(&self, req: RequestInfo, file: T, size: u32, callback: ReplyCb<Vec<u8>>) {
        callback(self.get_fuse_handler().listxattr(req, file, size));
    }

    fn removexattr(&self, req: RequestInfo, file: T, name: &OsStr, callback: ReplyCb<()>) {
        callback(self.get_fuse_handler().removexattr(req, file, name));
    }

    fn access(&self, req: RequestInfo, file: T, mask: AccessMask, callback: ReplyCb<()>) {
        callback(self.get_fuse_handler().access(req, file, mask));
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
        callback(
            self.get_fuse_handler()
                .getlk(req, file, file_handle, lock_owner, lock_info),
        );
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
        callback(self.get_fuse_handler().setlk(
            req,
            file,
            file_handle,
            lock_owner,
            lock_info,
            sleep,
        ));
    }

    fn bmap(&self, req: RequestInfo, file: T, blocksize: u32, idx: u64, callback: ReplyCb<u64>) {
        callback(self.get_fuse_handler().bmap(req, file, blocksize, idx));
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
        callback(self.get_fuse_handler().ioctl(
            req,
            file,
            file_handle,
            flags,
            cmd,
            in_data,
            out_size,
        ));
    }

    fn create(
        &self,
        req: RequestInfo,
        parent: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
        callback: ReplyCb<(FileHandle, FileAttribute, FUSEOpenResponseFlags)>,
    ) {
        callback(
            self.get_fuse_handler()
                .create(req, parent, name, mode, umask, flags),
        );
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
        callback(
            self.get_fuse_handler()
                .fallocate(req, file, file_handle, offset, length, mode),
        );
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
        callback(
            self.get_fuse_handler()
                .lseek(req, file, file_handle, offset, whence),
        );
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
        callback(self.get_fuse_handler().copy_file_range(
            req,
            file_in,
            file_handle_in,
            offset_in,
            file_out,
            file_handle_out,
            offset_out,
            len,
            flags,
        ));
    }
}
