use std::ffi::{NulError, OsString};
use std::io;
use std::time::{Duration, SystemTime};

use fuser::{FileAttr, Request, TimeOrNow};

pub use fuser::FileType;

/// # Errors
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PosixError(i32);

impl PosixError {
    pub const PERMISSION_DENIED: PosixError = PosixError(libc::EPERM);
    pub const FILE_NOT_FOUND: PosixError = PosixError(libc::ENOENT);
    pub const NO_SUCH_PROCESS: PosixError = PosixError(libc::ESRCH);
    pub const INTERRUPTED_SYSTEM_CALL: PosixError = PosixError(libc::EINTR);
    pub const INPUT_OUTPUT_ERROR: PosixError = PosixError(libc::EIO);
    pub const NO_SUCH_DEVICE_OR_ADDRESS: PosixError = PosixError(libc::ENXIO);
    pub const ARGUMENT_LIST_TOO_LONG: PosixError = PosixError(libc::E2BIG);
    pub const EXEC_FORMAT_ERROR: PosixError = PosixError(libc::ENOEXEC);
    pub const BAD_FILE_DESCRIPTOR: PosixError = PosixError(libc::EBADF);
    pub const NO_CHILD_PROCESSES: PosixError = PosixError(libc::ECHILD);
    pub const RESOURCE_DEADLOCK_AVOIDED: PosixError = PosixError(libc::EDEADLK);
    pub const OUT_OF_MEMORY: PosixError = PosixError(libc::ENOMEM);
    pub const PERMISSION_DENIED_ACCESS: PosixError = PosixError(libc::EACCES);
    pub const BAD_ADDRESS: PosixError = PosixError(libc::EFAULT);
    pub const BLOCK_DEVICE_REQUIRED: PosixError = PosixError(libc::ENOTBLK);
    pub const DEVICE_OR_RESOURCE_BUSY: PosixError = PosixError(libc::EBUSY);
    pub const FILE_EXISTS: PosixError = PosixError(libc::EEXIST);
    pub const INVALID_CROSS_DEVICE_LINK: PosixError = PosixError(libc::EXDEV);
    pub const NO_SUCH_DEVICE: PosixError = PosixError(libc::ENODEV);
    pub const NOT_A_DIRECTORY: PosixError = PosixError(libc::ENOTDIR);
    pub const IS_A_DIRECTORY: PosixError = PosixError(libc::EISDIR);
    pub const INVALID_ARGUMENT: PosixError = PosixError(libc::EINVAL);
    pub const TOO_MANY_OPEN_FILES: PosixError = PosixError(libc::EMFILE);
    pub const TOO_MANY_FILES_IN_SYSTEM: PosixError = PosixError(libc::ENFILE);
    pub const INAPPROPRIATE_IOCTL_FOR_DEVICE: PosixError = PosixError(libc::ENOTTY);
    pub const TEXT_FILE_BUSY: PosixError = PosixError(libc::ETXTBSY);
    pub const FILE_TOO_LARGE: PosixError = PosixError(libc::EFBIG);
    pub const NO_SPACE_LEFT_ON_DEVICE: PosixError = PosixError(libc::ENOSPC);
    pub const ILLEGAL_SEEK: PosixError = PosixError(libc::ESPIPE);
    pub const READ_ONLY_FILE_SYSTEM: PosixError = PosixError(libc::EROFS);
    pub const TOO_MANY_LINKS: PosixError = PosixError(libc::EMLINK);
    pub const BROKEN_PIPE: PosixError = PosixError(libc::EPIPE);
    pub const DOMAIN_ERROR: PosixError = PosixError(libc::EDOM);
    pub const RESULT_TOO_LARGE: PosixError = PosixError(libc::ERANGE);
    pub const RESOURCE_UNAVAILABLE_TRY_AGAIN: PosixError = PosixError(libc::EAGAIN);
    pub const OPERATION_WOULD_BLOCK: PosixError = PosixError(libc::EWOULDBLOCK);
    pub const OPERATION_IN_PROGRESS: PosixError = PosixError(libc::EINPROGRESS);
    pub const OPERATION_ALREADY_IN_PROGRESS: PosixError = PosixError(libc::EALREADY);
    pub const NOT_A_SOCKET: PosixError = PosixError(libc::ENOTSOCK);
    pub const MESSAGE_SIZE: PosixError = PosixError(libc::EMSGSIZE);
    pub const PROTOCOL_WRONG_TYPE: PosixError = PosixError(libc::EPROTOTYPE);
    pub const PROTOCOL_NOT_AVAILABLE: PosixError = PosixError(libc::ENOPROTOOPT);
    pub const PROTOCOL_NOT_SUPPORTED: PosixError = PosixError(libc::EPROTONOSUPPORT);
    pub const SOCKET_TYPE_NOT_SUPPORTED: PosixError = PosixError(libc::ESOCKTNOSUPPORT);
    pub const OPERATION_NOT_SUPPORTED: PosixError = PosixError(libc::EOPNOTSUPP);
    pub const PROTOCOL_FAMILY_NOT_SUPPORTED: PosixError = PosixError(libc::EPFNOSUPPORT);
    pub const ADDRESS_FAMILY_NOT_SUPPORTED: PosixError = PosixError(libc::EAFNOSUPPORT);
    pub const ADDRESS_IN_USE: PosixError = PosixError(libc::EADDRINUSE);
    pub const ADDRESS_NOT_AVAILABLE: PosixError = PosixError(libc::EADDRNOTAVAIL);
    pub const NETWORK_DOWN: PosixError = PosixError(libc::ENETDOWN);
    pub const NETWORK_UNREACHABLE: PosixError = PosixError(libc::ENETUNREACH);
    pub const NETWORK_RESET: PosixError = PosixError(libc::ENETRESET);
    pub const CONNECTION_ABORTED: PosixError = PosixError(libc::ECONNABORTED);
    pub const CONNECTION_RESET: PosixError = PosixError(libc::ECONNRESET);
    pub const NO_BUFFER_SPACE_AVAILABLE: PosixError = PosixError(libc::ENOBUFS);
    pub const ALREADY_CONNECTED: PosixError = PosixError(libc::EISCONN);
    pub const NOT_CONNECTED: PosixError = PosixError(libc::ENOTCONN);
    pub const DESTINATION_ADDRESS_REQUIRED: PosixError = PosixError(libc::EDESTADDRREQ);
    pub const SHUTDOWN: PosixError = PosixError(libc::ESHUTDOWN);
    pub const TOO_MANY_REFERENCES: PosixError = PosixError(libc::ETOOMANYREFS);
    pub const TIMED_OUT: PosixError = PosixError(libc::ETIMEDOUT);
    pub const CONNECTION_REFUSED: PosixError = PosixError(libc::ECONNREFUSED);
    pub const TOO_MANY_SYMBOLIC_LINKS: PosixError = PosixError(libc::ELOOP);
    pub const FILE_NAME_TOO_LONG: PosixError = PosixError(libc::ENAMETOOLONG);
    pub const HOST_IS_DOWN: PosixError = PosixError(libc::EHOSTDOWN);
    pub const NO_ROUTE_TO_HOST: PosixError = PosixError(libc::EHOSTUNREACH);
    pub const DIRECTORY_NOT_EMPTY: PosixError = PosixError(libc::ENOTEMPTY);
    pub const TOO_MANY_USERS: PosixError = PosixError(libc::EUSERS);
    pub const QUOTA_EXCEEDED: PosixError = PosixError(libc::EDQUOT);
    pub const STALE_FILE_HANDLE: PosixError = PosixError(libc::ESTALE);
    pub const OBJECT_IS_REMOTE: PosixError = PosixError(libc::EREMOTE);
    pub const NO_LOCKS_AVAILABLE: PosixError = PosixError(libc::ENOLCK);
    pub const FUNCTION_NOT_IMPLEMENTED: PosixError = PosixError(libc::ENOSYS);
    pub const LIBRARY_ERROR: PosixError = PosixError(libc::ELIBEXEC);
    pub const NOT_SUPPORTED: PosixError = PosixError(libc::ENOTSUP);
    pub const ILLEGAL_BYTE_SEQUENCE: PosixError = PosixError(libc::EILSEQ);
    pub const BAD_MESSAGE: PosixError = PosixError(libc::EBADMSG);
    pub const IDENTIFIER_REMOVED: PosixError = PosixError(libc::EIDRM);
    pub const MULTIHOP_ATTEMPTED: PosixError = PosixError(libc::EMULTIHOP);
    pub const NO_DATA_AVAILABLE: PosixError = PosixError(libc::ENODATA);
    pub const LINK_HAS_BEEN_SEVERED: PosixError = PosixError(libc::ENOLINK);
    pub const NO_MESSAGE: PosixError = PosixError(libc::ENOMSG);
    pub const OUT_OF_STREAMS: PosixError = PosixError(libc::ENOSR);
}

impl From<PosixError> for io::Error {
    fn from(value: PosixError) -> Self {
        Self::from_raw_os_error(value.0)
    }
}

impl From<PosixError> for i32 {
    fn from(value: PosixError) -> Self {
        value.0
    }
}

impl From<NulError> for PosixError {
    fn from(_value: NulError) -> Self {
        PosixError::INVALID_ARGUMENT
    }
}

pub fn from_last_errno() -> io::Error {
    std::io::Error::last_os_error()
}

/// Represents the file handle of an open file in fuse filesystem
/// May not represent a valid file descriptor
#[derive(Debug, Clone)]
pub struct FileHandle(u64);

impl From<u64> for FileHandle {
    fn from(value: u64) -> Self {
        FileHandle(value)
    }
}

impl From<FileHandle> for u64 {
    fn from(value: FileHandle) -> Self {
        value.0
    }
}

/// Represents the file descriptor of an open file on the system
#[derive(Debug, Clone)]
pub struct FileDescriptor(i32);

impl From<FileDescriptor> for i32 {
    fn from(value: FileDescriptor) -> Self {
        value.0
    }
}

impl From<i32> for FileDescriptor {
    fn from(value: i32) -> Self {
        FileDescriptor(value)
    }
}

impl TryFrom<FileHandle> for FileDescriptor {
    type Error = io::Error;

    fn try_from(fh: FileHandle) -> Result<Self, Self::Error> {
        Ok(Self(
            i32::try_from(u64::from(fh)).map_err(|_| PosixError::INVALID_ARGUMENT)?,
        ))
    }
}

/// Must be called immediatly after an error to get meaningfull errno
pub fn to_file_handle(fd: FileDescriptor) -> Result<FileHandle, io::Error> {
    let fd: i32 = fd.into();
    if fd < 0 {
        return Err(from_last_errno());
    }
    return Ok(FileHandle::from(fd as u64));
}

/// Abstraction for seek whence
#[derive(Debug, Clone, Copy)]
pub enum Whence {
    Start,
    Current,
    End,
}

impl From<i32> for Whence {
    fn from(value: i32) -> Self {
        match value {
            libc::SEEK_SET => Whence::Start,
            libc::SEEK_CUR => Whence::Current,
            libc::SEEK_END => Whence::End,
            _ => panic!("Invalid whence"),
        }
    }
}

impl From<Whence> for i32 {
    fn from(value: Whence) -> Self {
        match value {
            Whence::Start => libc::SEEK_SET,
            Whence::Current => libc::SEEK_CUR,
            Whence::End => libc::SEEK_END,
        }
    }
}

/// Abstraction over posix rdev (dev number of the root device) that define device type
#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    RegularFile,
    Directory,
    CharacterDevice { major: u32, minor: u32 },
    BlockDevice { major: u32, minor: u32 },
    NamedPipe,
    Socket,
    Symlink,
    Unknown,
}

impl DeviceType {
    pub fn from_rdev(rdev: u32) -> Self {
        use libc::*;
        // Extract major and minor device numbers (assuming the device number format).
        let major = rdev >> 8; // Major is the upper part of the 32-bit value
        let minor = rdev & 0xFF; // Minor is the lower 8 bits
        match rdev {
            x if x & S_IFREG != 0 => DeviceType::RegularFile,
            x if x & S_IFDIR != 0 => DeviceType::Directory,
            x if x & S_IFCHR != 0 => DeviceType::CharacterDevice { major, minor },
            x if x & S_IFBLK != 0 => DeviceType::BlockDevice { major, minor },
            x if x & S_IFIFO != 0 => DeviceType::NamedPipe,
            x if x & S_IFSOCK != 0 => DeviceType::Socket,
            x if x & S_IFLNK != 0 => DeviceType::Symlink,
            _ => DeviceType::Unknown,
        }
    }

    pub fn to_rdev(&self) -> u32 {
        use libc::*;

        match self {
            DeviceType::RegularFile => S_IFREG,
            DeviceType::Directory => S_IFDIR,
            DeviceType::CharacterDevice { major, minor } => (major << 8) | (minor & 0xFF) | S_IFCHR,
            DeviceType::BlockDevice { major, minor } => (major << 8) | (minor & 0xFF) | S_IFBLK,
            DeviceType::NamedPipe => S_IFIFO,
            DeviceType::Socket => S_IFSOCK,
            DeviceType::Symlink => S_IFLNK,
            DeviceType::Unknown => 0, // Represents an unknown device
        }
    }
}

#[derive(Debug, Clone)]
pub struct StatFs {
    pub total_blocks: u64,        // Total number of blocks
    pub free_blocks: u64,         // Number of free blocks
    pub available_blocks: u64,    // Number of blocks available to non-root users
    pub total_files: u64,         // Total number of files
    pub free_files: u64,          // Number of free file nodes
    pub block_size: u32,          // Size of a block in bytes
    pub max_filename_length: u32, // Maximum length of a filename
    pub fragment_size: u32,       // Fragment size in bytes
}

impl StatFs {
    /// Default value that should work on most systems
    /// See also helper::statfs for the real stuff
    pub fn default() -> Self {
        StatFs {
            total_blocks: u64::MAX,
            free_blocks: u64::MAX,
            available_blocks: u64::MAX,
            total_files: u64::MAX / 2,
            free_files: u64::MAX / 2,
            block_size: 4096,
            max_filename_length: 255,
            fragment_size: 4096,
        }
    }
}

// Flag utility
macro_rules! impl_flag_methods {
    ($struct_name:ident, $inner_type:ty) => {
        impl $struct_name {
            pub fn new() -> Self {
                Self(0)
            }

            pub fn from(flags: $inner_type) -> Self {
                Self(flags)
            }

            pub fn add_flag(mut self, flag: $inner_type) -> Self {
                self.0 |= flag;
                self
            }

            pub fn contains(&self, flag: $inner_type) -> bool {
                (self.0 & flag) != 0
            }

            pub fn as_raw(&self) -> $inner_type {
                self.0
            }
        }

        impl std::ops::BitOr for $struct_name {
            type Output = Self;

            fn bitor(self, rhs: Self) -> Self::Output {
                Self(self.0 | rhs.0)
            }
        }

        impl std::ops::BitAnd for $struct_name {
            type Output = Self;

            fn bitand(self, rhs: Self) -> Self::Output {
                Self(self.0 & rhs.0)
            }
        }
    };
}

/// # Posix Flags

#[derive(Debug, Copy, Clone)]
pub struct OpenFlags(i32);
impl_flag_methods!(OpenFlags, i32);

impl OpenFlags {
    pub const READ_ONLY: OpenFlags = OpenFlags(libc::O_RDONLY);
    pub const WRITE_ONLY: OpenFlags = OpenFlags(libc::O_WRONLY);
    pub const READ_WRITE: OpenFlags = OpenFlags(libc::O_RDWR);
    pub const CREATE: OpenFlags = OpenFlags(libc::O_CREAT);
    pub const CREATE_EXCLUSIVE: OpenFlags = OpenFlags(libc::O_EXCL);
    pub const NO_TERMINAL_CONTROL: OpenFlags = OpenFlags(libc::O_NOCTTY);
    pub const TRUNCATE: OpenFlags = OpenFlags(libc::O_TRUNC);
    pub const APPEND_MODE: OpenFlags = OpenFlags(libc::O_APPEND);
    pub const NON_BLOCKING_MODE: OpenFlags = OpenFlags(libc::O_NONBLOCK);
    pub const SYNC_DATA_ONLY: OpenFlags = OpenFlags(libc::O_DSYNC);
    pub const SYNC_DATA_AND_METADATA: OpenFlags = OpenFlags(libc::O_SYNC);
    pub const SYNC_READS_AND_WRITES: OpenFlags = OpenFlags(libc::O_RSYNC);
    pub const MUST_BE_DIRECTORY: OpenFlags = OpenFlags(libc::O_DIRECTORY);
    pub const DO_NOT_FOLLOW_SYMLINKS: OpenFlags = OpenFlags(libc::O_NOFOLLOW);
    pub const CLOSE_ON_EXEC: OpenFlags = OpenFlags(libc::O_CLOEXEC);
    pub const TEMPORARY_FILE: OpenFlags = OpenFlags(libc::O_TMPFILE);
}

#[derive(Debug, Copy, Clone)]
pub struct RenameFlags(u32);
impl_flag_methods!(RenameFlags, u32);

impl RenameFlags {
    pub const EXCHANGE: RenameFlags = RenameFlags(libc::RENAME_EXCHANGE);
    pub const NOREPLACE: RenameFlags = RenameFlags(libc::RENAME_NOREPLACE);
}

#[derive(Debug, Copy, Clone)]
pub struct IOCtlFlags(u32);

// Define the flag methods using the macro
impl_flag_methods!(IOCtlFlags, u32);

impl IOCtlFlags {
    // Placeholder for future flags
}

#[derive(Debug, Copy, Clone)]
pub struct LockType(i32);

// Define the flag methods using the macro
impl_flag_methods!(LockType, i32);

impl LockType {
    pub const UNLOCKED: LockType = LockType(libc::F_UNLCK);
    pub const READ_LOCK: LockType = LockType(libc::F_RDLCK);
    pub const WRITE_LOCK: LockType = LockType(libc::F_WRLCK);
}

#[derive(Debug, Copy, Clone)]
pub struct AccessMask(i32);
impl_flag_methods!(AccessMask, i32);

impl AccessMask {
    pub const EXISTS: AccessMask = AccessMask(libc::F_OK);
    pub const CAN_READ: AccessMask = AccessMask(libc::R_OK);
    pub const CAN_WRITE: AccessMask = AccessMask(libc::W_OK);
    pub const CAN_EXEC: AccessMask = AccessMask(libc::X_OK);
}

/// # Fuse Flags
#[derive(Debug, Copy, Clone)]
pub struct FUSEOpenResponseFlags(u32);
impl_flag_methods!(FUSEOpenResponseFlags, u32);

impl FUSEOpenResponseFlags {
    pub const DIRECT_IO: FUSEOpenResponseFlags = FUSEOpenResponseFlags(1 << 0);
    pub const KEEP_CACHE: FUSEOpenResponseFlags = FUSEOpenResponseFlags(1 << 1);
    pub const NONSEEKABLE: FUSEOpenResponseFlags = FUSEOpenResponseFlags(1 << 2);
    pub const CACHE_DIR: FUSEOpenResponseFlags = FUSEOpenResponseFlags(1 << 3);
    pub const STREAM: FUSEOpenResponseFlags = FUSEOpenResponseFlags(1 << 4);
    pub const NOFLUSH: FUSEOpenResponseFlags = FUSEOpenResponseFlags(1 << 5);
    pub const PARALLEL_DIRECT_WRITES: FUSEOpenResponseFlags = FUSEOpenResponseFlags(1 << 6);
    pub const PASSTHROUGH: FUSEOpenResponseFlags = FUSEOpenResponseFlags(1 << 7);
}

#[derive(Debug, Copy, Clone)]
pub struct FUSEReleaseFlags(i32);
impl_flag_methods!(FUSEReleaseFlags, i32);

impl FUSEReleaseFlags {
    pub const FLUSH: FUSEReleaseFlags = FUSEReleaseFlags(1 << 0);
    pub const FLOCK_UNLOCK: FUSEReleaseFlags = FUSEReleaseFlags(1 << 1);
}

#[derive(Debug, Copy, Clone)]
pub struct FUSEGetAttrFlags(i32);
impl_flag_methods!(FUSEGetAttrFlags, i32);

impl FUSEGetAttrFlags {
    pub const GETATTR_FH: FUSEGetAttrFlags = FUSEGetAttrFlags(1 << 0);
}

#[derive(Debug, Copy, Clone)]
pub struct FUSEWriteFlags(u32);
impl_flag_methods!(FUSEWriteFlags, u32);

impl FUSEWriteFlags {
    pub const CACHE: FUSEWriteFlags = FUSEWriteFlags(1 << 0);
    pub const LOCKOWNER: FUSEWriteFlags = FUSEWriteFlags(1 << 1);
    pub const KILL_SUIDGID: FUSEWriteFlags = FUSEWriteFlags(1 << 2);
}

#[derive(Debug, Copy, Clone)]
pub struct FUSEReadFlags(i32);
impl_flag_methods!(FUSEReadFlags, i32);

impl FUSEReadFlags {
    pub const LOCKOWNER: FUSEReadFlags = FUSEReadFlags(1 << 1);
}

#[derive(Debug, Copy, Clone)]
pub struct FUSEIoctlFlags(u32);
impl_flag_methods!(FUSEIoctlFlags, u32);

impl FUSEIoctlFlags {
    pub const COMPAT: FUSEIoctlFlags = FUSEIoctlFlags(1 << 0);
    pub const UNRESTRICTED: FUSEIoctlFlags = FUSEIoctlFlags(1 << 1);
    pub const RETRY: FUSEIoctlFlags = FUSEIoctlFlags(1 << 2);
    pub const IOCTL_32BIT: FUSEIoctlFlags = FUSEIoctlFlags(1 << 3);
    pub const DIR: FUSEIoctlFlags = FUSEIoctlFlags(1 << 4);
    pub const COMPAT_X32: FUSEIoctlFlags = FUSEIoctlFlags(1 << 5);
}

#[derive(Debug, Copy, Clone)]
pub struct FUSEFsyncFlags(u32);
impl_flag_methods!(FUSEFsyncFlags, u32);

impl FUSEFsyncFlags {
    pub const FDATASYNC: FUSEFsyncFlags = FUSEFsyncFlags(1 << 0);
}

#[derive(Debug, Copy, Clone)]
pub struct FUSEAttrFlags(u32);
impl_flag_methods!(FUSEAttrFlags, u32);

impl FUSEAttrFlags {
    pub const SUBMOUNT: FUSEAttrFlags = FUSEAttrFlags(1 << 0);
    pub const DAX: FUSEAttrFlags = FUSEAttrFlags(1 << 1);
}

#[derive(Debug, Copy, Clone)]
pub struct FUSEOpenFlags(i32);
impl_flag_methods!(FUSEOpenFlags, i32);

impl FUSEOpenFlags {
    pub const KILL_SUIDGID: FUSEOpenFlags = FUSEOpenFlags(1 << 0);
}

#[derive(Debug, Copy, Clone)]
pub struct FUSESetXAttrFlags(i32);
impl_flag_methods!(FUSESetXAttrFlags, i32);

impl FUSESetXAttrFlags {
    pub const ACL_KILL_SGID: FUSESetXAttrFlags = FUSESetXAttrFlags(1 << 0);
}

// Fuse related structs
#[derive(Debug)]
pub struct RequestInfo {
    pub id: u64,
    pub uid: u32,
    pub gid: u32,
    pub pid: u32,
    // Other request-specific metadata
}
impl<'a> From<&Request<'a>> for RequestInfo {
    fn from(req: &Request<'a>) -> Self {
        Self {
            id: req.unique(),
            uid: req.uid(),
            gid: req.gid(),
            pid: req.pid(),
        }
    }
}

/// Fuse can cache file attributes. AttributeResponse allow to fine tune the value
/// Otherwise, ttl will be set by FilesystemAPI::get_default_ttl(), and generation will be random
#[derive(Debug, PartialEq)]
pub struct AttributeResponse {
    pub file_attr: FileAttr,
    pub ttl: Option<Duration>,
    pub generation: Option<u64>,
}

impl AttributeResponse {
    pub fn new(file_attr: FileAttr, ttl: Option<Duration>, generation: Option<u64>) -> Self {
        Self {
            file_attr,
            ttl,
            generation,
        }
    }
}

impl From<FileAttr> for AttributeResponse {
    fn from(value: FileAttr) -> Self {
        Self {
            file_attr: value,
            ttl: None,
            generation: None,
        }
    }
}

#[derive(Debug)]
pub struct FuseDirEntry {
    pub inode: u64,
    pub name: OsString,
    pub kind: FileType,
}

#[derive(Debug)]
pub struct FuseDirEntryPlus {
    pub inode: u64,
    pub name: OsString,
    pub attr_response: AttributeResponse,
}

#[derive(Debug)]
pub struct SetAttrRequest {
    pub mode: Option<u32>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub size: Option<u64>,
    pub atime: Option<TimeOrNow>,
    pub mtime: Option<TimeOrNow>,
    pub ctime: Option<SystemTime>,
    pub crtime: Option<SystemTime>,
    pub chgtime: Option<SystemTime>,
    pub bkuptime: Option<SystemTime>,
    pub flags: Option<()>, // Unused
    pub file_handle: Option<FileHandle>,
}

impl SetAttrRequest {
    pub fn new() -> Self {
        Self {
            mode: None,
            uid: None,
            gid: None,
            size: None,
            atime: None,
            mtime: None,
            ctime: None,
            crtime: None,
            chgtime: None,
            bkuptime: None,
            flags: None,
            file_handle: None,
        }
    }

    pub fn mode(mut self, mode: u32) -> Self {
        self.mode = Some(mode);
        self
    }

    pub fn uid(mut self, uid: u32) -> Self {
        self.uid = Some(uid);
        self
    }

    pub fn gid(mut self, gid: u32) -> Self {
        self.gid = Some(gid);
        self
    }

    pub fn size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn atime(mut self, atime: TimeOrNow) -> Self {
        self.atime = Some(atime);
        self
    }

    pub fn mtime(mut self, mtime: TimeOrNow) -> Self {
        self.mtime = Some(mtime);
        self
    }

    pub fn ctime(mut self, ctime: SystemTime) -> Self {
        self.ctime = Some(ctime);
        self
    }

    pub fn crtime(mut self, crtime: SystemTime) -> Self {
        self.crtime = Some(crtime);
        self
    }

    pub fn chgtime(mut self, chgtime: SystemTime) -> Self {
        self.chgtime = Some(chgtime);
        self
    }

    pub fn bkuptime(mut self, bkuptime: SystemTime) -> Self {
        self.bkuptime = Some(bkuptime);
        self
    }

    /// Unused by FUSE
    pub fn flags(mut self, flags: ()) -> Self {
        self.flags = Some(flags);
        self
    }

    pub fn file_handle(mut self, file_handle: FileHandle) -> Self {
        self.file_handle = Some(file_handle);
        self
    }
}

#[derive(Debug)]
pub struct LockInfo {
    pub start: u64,
    pub end: u64,
    pub lock_type: LockType,
    pub pid: u32,
}
