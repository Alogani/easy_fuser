use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::time::Duration;

use crate::types::*;

// Documentation was taken from https://docs.rs/fuser

/// This trait must be implemented to provide a userspace filesystem via FUSE. These methods correspond to fuse_lowlevel_ops in libfuse.

pub trait FuseHandler<T: FileIdType>: 'static {
    /// Delegate unprovided methods to another FuseHandler, mimicking inheritance
    fn get_inner(&self) -> &Box<dyn FuseHandler<T>>;

    /// Provide a default TTL if not directly in response T::Metadatas
    fn get_default_ttl(&self) -> Duration {
        Duration::from_secs(1)
    }

    /// Initialize filesystem. Called before any other filesystem method. The kernel module connection can be configured using the KernelConfig object
    fn init(&self, req: &RequestInfo, config: &mut KernelConfig) -> FuseResult<()> {
        self.get_inner().init(req, config)
    }

    /// Clean up filesystem. Called on filesystem exit.
    fn destroy(&self) {
        self.get_inner().destroy();
    }

    /// Look up a directory entry by name and get its attributes.
    fn lookup(&self, req: &RequestInfo, parent_id: T, name: &OsStr) -> FuseResult<T::Metadata> {
        self.get_inner().lookup(req, parent_id, name)
    }

    /// Forget about an inode. The nlookup parameter indicates the number of lookups previously performed on this inode. If the filesystem implements inode lifetimes, it is recommended that inodes acquire a single reference on each lookup, and lose nlookup references on each forget. The filesystem may ignore forget calls, if the inodes don’t need to have a limited lifetime. On unmount it is not guaranteed, that all referenced inodes will receive a forget message.
    fn forget(&self, req: &RequestInfo, file_id: T, nlookup: u64) {
        self.get_inner().forget(req, file_id, nlookup);
    }

    /// Get file attributes.
    fn getattr(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: Option<FileHandle>,
    ) -> FuseResult<FileAttribute> {
        self.get_inner().getattr(req, file_id, file_handle)
    }

    /// Set file attributes.
    fn setattr(
        &self,
        req: &RequestInfo,
        file_id: T,
        attrs: SetAttrRequest,
    ) -> FuseResult<FileAttribute> {
        self.get_inner().setattr(req, file_id, attrs)
    }

    /// Read symbolic link.
    fn readlink(&self, req: &RequestInfo, file_id: T) -> FuseResult<Vec<u8>> {
        self.get_inner().readlink(req, file_id)
    }

    /// Create file node. For example: a regular file, character device, block device, fifo or socket.
    fn mknod(
        &self,
        req: &RequestInfo,
        parent_id: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: DeviceType,
    ) -> FuseResult<T::Metadata> {
        self.get_inner()
            .mknod(req, parent_id, name, mode, umask, rdev)
    }

    /// Create a directory.
    fn mkdir(
        &self,
        req: &RequestInfo,
        parent_id: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
    ) -> FuseResult<T::Metadata> {
        self.get_inner().mkdir(req, parent_id, name, mode, umask)
    }

    /// Remove a file.
    fn unlink(&self, req: &RequestInfo, parent_id: T, name: &OsStr) -> FuseResult<()> {
        self.get_inner().unlink(req, parent_id, name)
    }

    /// Remove a directory.
    fn rmdir(&self, req: &RequestInfo, parent_id: T, name: &OsStr) -> FuseResult<()> {
        self.get_inner().rmdir(req, parent_id, name)
    }

    /// Create a symbolic link.
    fn symlink(
        &self,
        req: &RequestInfo,
        parent_id: T,
        link_name: &OsStr,
        target: &Path,
    ) -> FuseResult<T::Metadata> {
        self.get_inner().symlink(req, parent_id, link_name, target)
    }

    /// Rename a file.
    fn rename(
        &self,
        req: &RequestInfo,
        parent_id: T,
        name: &OsStr,
        newparent: T,
        newname: &OsStr,
        flags: RenameFlags,
    ) -> FuseResult<()> {
        self.get_inner()
            .rename(req, parent_id, name, newparent, newname, flags)
    }

    /// Create a hard link.
    fn link(
        &self,
        req: &RequestInfo,
        file_id: T,
        newparent: T,
        newname: &OsStr,
    ) -> FuseResult<T::Metadata> {
        self.get_inner().link(req, file_id, newparent, newname)
    }

    /// Open a file. Open flags (with the exception of O_CREAT, O_EXCL, O_NOCTTY and O_TRUNC) are available in flags. You may store an arbitrary file handle (pointer, index, etc) in file_handle response, and use this in other all other file operations (read, write, flush, release, fsync). Filesystem may also implement stateless file I/O and not store anything in fh. There are also some flags (direct_io, keep_cache) which the filesystem may set, to change the way the file is opened. See fuse_file_info structure in <fuse_common.h> for more details.
    fn open(
        &self,
        req: &RequestInfo,
        file_id: T,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        self.get_inner().open(req, file_id, flags)
    }

    /// Read data. Read should send exactly the number of bytes requested except on EOF or error, otherwise the rest of the data will be substituted with zeroes. An exception to this is when the file has been opened in ‘direct_io’ mode, in which case the return value of the read system call will reflect the return value of this operation. fh will contain the value set by the open method, or will be undefined if the open method didn’t set any value.
    ///
    /// flags: these are the file flags, such as O_SYNC. Only supported with ABI >= 7.9 lock_owner: only supported with ABI >= 7.9
    fn read(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        offset: i64,
        size: u32,
        flags: FUSEOpenFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        self.get_inner()
            .read(req, file_id, file_handle, offset, size, flags, lock_owner)
    }

    /// Write data. Write should return exactly the number of bytes requested except on error. An exception to this is when the file has been opened in ‘direct_io’ mode, in which case the return value of the write system call will reflect the return value of this operation. fh will contain the value set by the open method, or will be undefined if the open method didn’t set any value.
    ///
    /// write_flags: will contain FUSE_WRITE_CACHE, if this write is from the page cache. If set, the pid, uid, gid, and fh may not match the value that would have been sent if write cachin is disabled flags: these are the file flags, such as O_SYNC. Only supported with ABI >= 7.9 lock_owner: only supported with ABI >= 7.9
    fn write(
        &self,
        req: &RequestInfo,
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

    /// Flush method. This is called on each close() of the opened file. Since file descriptors can be duplicated (dup, dup2, fork), for one open call there may be many flush calls. Filesystems shouldn’t assume that flush will always be called after some writes, or that if will be called at all. fh will contain the value set by the open method, or will be undefined if the open method didn’t set any value. NOTE: the name of the method is misleading, since (unlike fsync) the filesystem is not forced to flush pending writes. One reason to flush data, is if the filesystem wants to return write errors. If the filesystem supports file locking operations (setlk, getlk) it should remove all locks belonging to ‘lock_owner’.
    fn flush(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        lock_owner: u64,
    ) -> FuseResult<()> {
        self.get_inner()
            .flush(req, file_id, file_handle, lock_owner)
    }

    /// Release an open file. Release is called when there are no more references to an open file: all file descriptors are closed and all memory mappings are unmapped. For every open call there will be exactly one release call. The filesystem may reply with an error, but error values are not returned to close() or munmap() which triggered the release. fh will contain the value set by the open method, or will be undefined if the open method didn’t set any value. flags will contain the same flags as for open.
    fn release(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        flags: OpenFlags,
        lock_owner: Option<u64>,
        flush: bool,
    ) -> FuseResult<()> {
        self.get_inner()
            .release(req, file_id, file_handle, flags, lock_owner, flush)
    }

    /// Synchronize file contents. If the datasync parameter is non-zero, then only the user data should be flushed, not the meta data.
    fn fsync(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        datasync: bool,
    ) -> FuseResult<()> {
        self.get_inner().fsync(req, file_id, file_handle, datasync)
    }

    /// Open a directory. Filesystem may store an arbitrary file handle (pointer, index, etc) in fh, and use this in other all other directory stream operations (readdir, releasedir, fsyncdir). Filesystem may also implement stateless directory I/O and not store anything in fh, though that makes it impossible to implement standard conforming directory stream operations in case the contents of the directory can change between opendir and releasedir.
    fn opendir(
        &self,
        req: &RequestInfo,
        file_id: T,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        self.get_inner().opendir(req, file_id, flags)
    }

    /// Read directory. file_handle will contain the value set by the opendir method, or will be undefined if the opendir method didn’t set any value.
    fn readdir(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
    ) -> FuseResult<Vec<(OsString, T::MinimalMetadata)>> {
        self.get_inner().readdir(req, file_id, file_handle)
    }

    // Read directory alongside its file attributes. A default implementation is provided by combining readdir and lookup, which should produce the same number of system calls
    fn readdirplus(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
    ) -> FuseResult<Vec<(OsString, T::Metadata)>> {
        let readdir_result = self.readdir(req, file_id, file_handle)?;
        let mut result = Vec::with_capacity(readdir_result.len());
        for (name, _) in readdir_result.into_iter() {
            result.push((name, self.lookup(req, file_id, &name)?));
        }
        Ok(result)
    }

    /// Release an open directory. For every opendir call there will be exactly one releasedir call. file_handle will contain the value set by the opendir method, or will be undefined if the opendir method didn’t set any value.
    fn releasedir(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        flags: OpenFlags,
    ) -> FuseResult<()> {
        self.get_inner()
            .releasedir(req, file_id, file_handle, flags)
    }

    /// Synchronize directory contents. If the datasync parameter is set, then only the directory contents should be flushed, not the meta data. file_handle will contain the value set by the opendir method, or will be undefined if the opendir method didn’t set any value.

    fn fsyncdir(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        datasync: bool,
    ) -> FuseResult<()> {
        self.get_inner()
            .fsyncdir(req, file_id, file_handle, datasync)
    }

    /// Get file system statistics.
    fn statfs(&self, req: &RequestInfo, file_id: T) -> FuseResult<StatFs> {
        self.get_inner().statfs(req, file_id)
    }

    /// Set an extended attribute.
    fn setxattr(
        &self,
        req: &RequestInfo,
        file_id: T,
        name: &OsStr,
        value: &[u8],
        flags: FUSESetXAttrFlags,
        position: u32,
    ) -> FuseResult<()> {
        self.get_inner()
            .setxattr(req, file_id, name, value, flags, position)
    }

    /// Get an extended attribute.
    fn getxattr(
        &self,
        req: &RequestInfo,
        file_id: T,
        name: &OsStr,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        self.get_inner().getxattr(req, file_id, name, size)
    }

    /// List extended attribute names.
    fn listxattr(&self, req: &RequestInfo, file_id: T, size: u32) -> FuseResult<Vec<u8>> {
        self.get_inner().listxattr(req, file_id, size)
    }

    /// Remove an extended attribute.
    fn removexattr(&self, req: &RequestInfo, file_id: T, name: &OsStr) -> FuseResult<()> {
        self.get_inner().removexattr(req, file_id, name)
    }

    /// Check file access permissions. This will be called for the access() system call. If the ‘default_permissions’ mount option is given, this method is not called. This method is not called under Linux kernel versions 2.4.x
    fn access(&self, req: &RequestInfo, file_id: T, mask: AccessMask) -> FuseResult<()> {
        self.get_inner().access(req, file_id, mask)
    }

    /// Test for a POSIX file lock.
    fn getlk(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
    ) -> FuseResult<LockInfo> {
        self.get_inner()
            .getlk(req, file_id, file_handle, lock_owner, lock_info)
    }

    /// Acquire, modify or release a POSIX file lock. For POSIX threads (NPTL) there’s a 1-1 relation between pid and owner, but otherwise this is not always the case. For checking lock ownership, ‘fi->owner’ must be used. The l_pid field in ‘struct flock’ should only be used to fill in this field in getlk(). Note: if the locking methods are not implemented, the kernel will still allow file locking to work locally. Hence these are only interesting for network filesystems and similar.
    fn setlk(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
        sleep: bool,
    ) -> FuseResult<()> {
        self.get_inner()
            .setlk(req, file_id, file_handle, lock_owner, lock_info, sleep)
    }

    /// Map block index within file to block index within device. Note: This makes sense only for block device backed filesystems mounted with the ‘blkdev’ option
    fn bmap(&self, req: &RequestInfo, file_id: T, blocksize: u32, idx: u64) -> FuseResult<u64> {
        self.get_inner().bmap(req, file_id, blocksize, idx)
    }

    /// control device
    fn ioctl(
        &self,
        req: &RequestInfo,
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

    /// Create and open a file. If the file does not exist, first create it with the specified mode, and then open it. Open flags (with the exception of O_NOCTTY) are available in flags.
    ///
    /// See the documentation of open for more informations about file_handle
    /// If this method is not implemented or under Linux kernel versions earlier than 2.6.15, the mknod() and open() methods will be called instead.
    fn create(
        &self,
        req: &RequestInfo,
        parent_id: T,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, T::Metadata, FUSEOpenResponseFlags)> {
        self.get_inner()
            .create(req, parent_id, name, mode, umask, flags)
    }

    /// Preallocate or deallocate space to a file
    fn fallocate(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        offset: i64,
        length: i64,
        mode: i32,
    ) -> FuseResult<()> {
        self.get_inner()
            .fallocate(req, file_id, file_handle, offset, length, mode)
    }

    /// Reposition read/write file offset
    fn lseek(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        offset: i64,
        whence: Whence,
    ) -> FuseResult<i64> {
        self.get_inner()
            .lseek(req, file_id, file_handle, offset, whence)
    }

    /// Copy the specified range from the source inode to the destination inode
    fn copy_file_range(
        &self,
        req: &RequestInfo,
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
