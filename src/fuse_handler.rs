/// The `FuseHandler` trait is the core interface for implementing a userspace filesystem via FUSE (Filesystem in Userspace).
///
/// This trait defines methods that correspond to various filesystem operations. By implementing this trait,
/// you can create custom filesystem behaviors for your FUSE-based filesystem.
///
/// # Type Parameter
///
/// - `T`: A type that implements `FileIdType`. This represents the unique identifier for files and directories in your filesystem.
///
/// # Usage
///
/// To create a custom filesystem, implement this trait for your struct. You can choose to implement all methods
/// or rely on default implementations provided by one of the provided templates (like `MirrorFs` or `DefaultFuseHandler`).
///
/// ## Example: Custom Filesystem with MirrorFs as base
///
/// ```rust, no_run
/// use easy_fuser::templates::{MirrorFsReadOnly, DefaultFuseHandler};
/// use easy_fuser::prelude::*;
/// use std::path::{Path, PathBuf};
/// use std::ffi::OsStr;
///
/// struct MyCustomFs {
///     inner: Box<dyn FuseHandler<PathBuf>>,
///     // other fields...
/// }
///
/// impl MyCustomFs {
///     pub fn new(source_path: PathBuf) -> Self {
///         MyCustomFs { inner: Box::new(MirrorFsReadOnly::new(source_path, DefaultFuseHandler::new())) }
///
///     }
/// }
///
/// impl FuseHandler<PathBuf> for MyCustomFs {
///     fn get_inner(&self) -> &dyn FuseHandler<PathBuf> {
///         // Delegate to MirrorFsReadOnly for standard behavior
///         self.inner.as_ref()
///     }
///
///     fn lookup(&self, req: &RequestInfo, parent_id: PathBuf, name: &OsStr) -> FuseResult<FileAttribute> {
///         // Custom logic for lookup operation
///         // ...
///
///         // Delegate to inner handler for standard behavior
///         self.inner.lookup(req, parent_id, name)
///     }
///
///     // Implement other FuseHandler methods as needed, delegating to self.inner as appropriate
///     // ...
/// }
/// ```
///
/// # Important Methods
///
/// While all methods in this trait are important for a fully functional filesystem, some key methods include:
///
/// - `lookup`: Look up a directory entry by name and get its attributes.
/// - `getattr`: Get file attributes.
/// - `read`: Read data from a file.
/// - `write`: Write data to a file.
/// - `readdir`: Read directory contents.
/// - `open`: Open a file.
/// - `create`: Create and open a file.
///
/// # Default Implementations
///
/// Many methods in this trait have default implementations that delegate to the inner handler returned by `get_inner()`.
/// This allows for easy extension and customization of existing filesystem implementations by chaining/overriding their behaviors.
///
/// # Thread Safety
///
/// This trait requires implementors to be `Send` and `Sync`, which is required for use with the FUSE library.
///
/// # Lifetime
///
/// The trait is bound by the `'static` lifetime, which is required for use with the FUSE library.
///
//// # Additional Resources:
/// For more detailed information, refer to the fuser project documentation, which serves as the foundation for this crate: https://docs.rs/fuser
///
/// Documentation is inspired by the original fuser documentation
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::time::Duration;

use crate::types::*;

pub trait FuseHandler<T: FileIdType>: Send + Sync + 'static {
    /// Delegate unprovided methods to another FuseHandler, enabling composition
    fn get_inner(&self) -> &dyn FuseHandler<T>;

    /// Provide a default Time-To-Live for file metadata
    ///
    /// Can be overriden for each FileAttributes returned.
    fn get_default_ttl(&self) -> Duration {
        Duration::from_secs(1)
    }

    /// Initialize the filesystem and configure kernel connection
    fn init(&self, req: &RequestInfo, config: &mut KernelConfig) -> FuseResult<()> {
        self.get_inner().init(req, config)
    }

    /// Perform cleanup operations on filesystem exit
    fn destroy(&self) {
        self.get_inner().destroy();
    }

    /// Retrieve file attributes for a directory entry by name and increment the lookup count associated with the inode.
    fn lookup(&self, req: &RequestInfo, parent_id: T, name: &OsStr) -> FuseResult<T::Metadata> {
        self.get_inner().lookup(req, parent_id, name)
    }

    /// Release references to an inode, if the nlookup count reaches zero (to substract from the number of lookups).
    fn forget(&self, req: &RequestInfo, file_id: T, nlookup: u64) {
        self.get_inner().forget(req, file_id, nlookup);
    }

    /// Modify file attributes
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

    /// Read the target of a symbolic link
    fn readlink(&self, req: &RequestInfo, file_id: T) -> FuseResult<Vec<u8>> {
        self.get_inner().readlink(req, file_id)
    }

    /// Create a new file node (regular file, device, FIFO, socket, etc)
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

    /// Create a new directory
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

    /// Remove a file
    fn unlink(&self, req: &RequestInfo, parent_id: T, name: &OsStr) -> FuseResult<()> {
        self.get_inner().unlink(req, parent_id, name)
    }

    /// Remove a directory
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

    /// Rename a file or directory
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

    /// Open a file and return a file handle.
    ///
    /// Open flags (with the exception of O_CREAT, O_EXCL, O_NOCTTY and O_TRUNC) are available in flags. You may store an arbitrary file handle (pointer, index, etc) in file_handle response, and use this in other all other file operations (read, write, flush, release, fsync). Filesystem may also implement stateless file I/O and not store anything in fh. There are also some flags (direct_io, keep_cache) which the filesystem may set, to change the way the file is opened. See fuse_file_info structure in <fuse_common.h> for more details.
    fn open(
        &self,
        req: &RequestInfo,
        file_id: T,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        self.get_inner().open(req, file_id, flags)
    }

    /// Read data from a file
    ///
    /// Read should send exactly the number of bytes requested except on EOF or error, otherwise the rest of the data will be substituted with zeroes. An exception to this is when the file has been opened in ‘direct_io’ mode, in which case the return value of the read system call will reflect the return value of this operation. fh will contain the value set by the open method, or will be undefined if the open method didn’t set any value.
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

    /// Write data to a file
    ///
    /// Write should return exactly the number of bytes requested except on error. An exception to this is when the file has been opened in ‘direct_io’ mode, in which case the return value of the write system call will reflect the return value of this operation. fh will contain the value set by the open method, or will be undefined if the open method didn’t set any value.
    ///
    /// write_flags: will contain FUSE_WRITE_CACHE, if this write is from the page cache. If set, the pid, uid, gid, and fh may not match the value that would have been sent if write cachin is disabled flags: these are the file flags, such as O_SYNC. Only supported with ABI >= 7.9 lock_owner: only supported with ABI >= 7.9
    fn write(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        offset: i64,
        data: Vec<u8>,
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

    /// Flush cached data for an open file
    ///
    /// Called on each close() of the opened file. Not guaranteed to be called after writes or at all.
    /// Used for returning write errors or removing file locks.
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

    /// Release an open file
    ///
    /// Called when all file descriptors are closed and all memory mappings are unmapped.
    /// Guaranteed to be called once for every open() call.
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

    /// Synchronize file contents
    ///
    /// If datasync is true, only flush user data, not metadata.
    fn fsync(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
        datasync: bool,
    ) -> FuseResult<()> {
        self.get_inner().fsync(req, file_id, file_handle, datasync)
    }

    /// Open a directory
    ///
    /// Allows storing a file handle for use in subsequent directory operations.
    fn opendir(
        &self,
        req: &RequestInfo,
        file_id: T,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        self.get_inner().opendir(req, file_id, flags)
    }

    /// Read directory contents
    ///
    /// Returns a list of directory entries with minimal metadata.
    fn readdir(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
    ) -> FuseResult<Vec<(OsString, T::MinimalMetadata)>> {
        self.get_inner().readdir(req, file_id, file_handle)
    }

    /// Read directory contents with full file attributes
    ///
    /// Default implementation combines readdir and lookup operations.
    fn readdirplus(
        &self,
        req: &RequestInfo,
        file_id: T,
        file_handle: FileHandle,
    ) -> FuseResult<Vec<(OsString, T::Metadata)>> {
        let readdir_result = self.readdir(req, file_id.clone(), file_handle)?;
        let mut result = Vec::with_capacity(readdir_result.len());
        for (name, _) in readdir_result.into_iter() {
            let metadata = self.lookup(req, file_id.clone(), &name)?;
            result.push((name, metadata));
        }
        Ok(result)
    }

    /// Release an open directory
    ///
    /// This method is called exactly once for every successful opendir operation.
    /// The file_handle parameter will contain the value set by the opendir method,
    /// or will be undefined if the opendir method didn't set any value.
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

    /// Synchronize directory contents
    ///
    /// If the datasync parameter is true, then only the directory contents should
    /// be flushed, not the metadata. The file_handle will contain the value set
    /// by the opendir method, or will be undefined if the opendir method didn't
    /// set any value.
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

    /// Get file system statistics
    fn statfs(&self, req: &RequestInfo, file_id: T) -> FuseResult<StatFs> {
        self.get_inner().statfs(req, file_id)
    }

    /// Set an extended attribute
    fn setxattr(
        &self,
        req: &RequestInfo,
        file_id: T,
        name: &OsStr,
        value: Vec<u8>,
        flags: FUSESetXAttrFlags,
        position: u32,
    ) -> FuseResult<()> {
        self.get_inner()
            .setxattr(req, file_id, name, value, flags, position)
    }

    /// Get an extended attribute
    fn getxattr(
        &self,
        req: &RequestInfo,
        file_id: T,
        name: &OsStr,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        self.get_inner().getxattr(req, file_id, name, size)
    }

    /// List extended attribute names
    fn listxattr(&self, req: &RequestInfo, file_id: T, size: u32) -> FuseResult<Vec<u8>> {
        self.get_inner().listxattr(req, file_id, size)
    }

    /// Remove an extended attribute.
    fn removexattr(&self, req: &RequestInfo, file_id: T, name: &OsStr) -> FuseResult<()> {
        self.get_inner().removexattr(req, file_id, name)
    }

    /// Check file access permissions
    ///
    /// This method is called for the access() system call. If the 'default_permissions'
    /// mount option is given, this method is not called. This method is not called
    /// under Linux kernel versions 2.4.x
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

    /// Acquire, modify or release a POSIX file lock
    ///
    /// For POSIX threads (NPTL) there's a 1-1 relation between pid and owner, but
    /// otherwise this is not always the case. For checking lock ownership, 'fi->owner'
    /// must be used. The l_pid field in 'struct flock' should only be used to fill
    /// in this field in getlk().
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

    /// Map block index within file to block index within device
    ///
    /// Note: This makes sense only for block device backed filesystems mounted
    /// with the 'blkdev' option
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
        in_data: Vec<u8>,
        out_size: u32,
    ) -> FuseResult<(i32, Vec<u8>)> {
        self.get_inner()
            .ioctl(req, file_id, file_handle, flags, cmd, in_data, out_size)
    }

    /// Create and open a file
    ///
    /// If the file does not exist, first create it with the specified mode, and then
    /// open it. Open flags (with the exception of O_NOCTTY) are available in flags.
    /// If this method is not implemented or under Linux kernel versions earlier than
    /// 2.6.15, the mknod() and open() methods will be called instead.
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
