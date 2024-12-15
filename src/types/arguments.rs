use std::ffi::OsString;
use std::io;
use std::time::{Duration, SystemTime};

use fuser::FileAttr as FuseFileAttr;
use fuser::{FileType, Request, TimeOrNow};

use super::{errors::*, LockType};

pub trait IdType: Send + std::fmt::Debug + 'static {}

#[derive(Debug, Clone, PartialEq)]
pub struct Inode(u64);
pub const INVALID_INODE: Inode = Inode(0);

impl From<u64> for Inode {
    fn from(value: u64) -> Self {
        Inode(value)
    }
}

impl From<Inode> for u64 {
    fn from(value: Inode) -> Self {
        value.0
    }
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

impl FileDescriptor {
    pub fn to_file_handle(self) -> Result<FileHandle, io::Error> {
        let fd: i32 = self.into();
        if fd < 0 {
            return Err(from_last_errno());
        }
        return Ok(FileHandle::from(fd as u64));
    }
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
pub struct FileAttribute {
    pub inode: Inode,
    pub size: u64,
    pub blocks: u64,
    pub atime: SystemTime,
    pub mtime: SystemTime,
    pub ctime: SystemTime,
    pub crtime: SystemTime,
    pub kind: FileType,
    pub perm: u16,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u32,
    pub blksize: u32,
    pub flags: u32,
    pub ttl: Option<Duration>,
    pub generation: Option<u64>,
}

impl FileAttribute {
    pub fn to_fuse(self) -> FuseFileAttr {
        FuseFileAttr {
            ino: self.inode.into(),
            size: self.size,
            blocks: self.blocks,
            atime: self.atime,
            mtime: self.mtime,
            ctime: self.ctime,
            crtime: self.crtime,
            kind: self.kind,
            perm: self.perm,
            nlink: self.nlink,
            uid: self.uid,
            gid: self.gid,
            rdev: self.rdev,
            blksize: self.blksize,
            flags: self.flags,
        }
    }
}

#[derive(Debug)]
pub struct FuseDirEntry {
    pub inode: Inode,
    pub name: OsString,
    pub kind: FileType,
}

#[derive(Debug)]
pub struct FuseDirEntryPlus {
    pub inode: Inode,
    pub name: OsString,
    pub attr: FileAttribute,
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
