//! Flags used in FUSE (Filesystem in Userspace) operations.
//!
//! This module defines various flag sets used throughout FUSE filesystem operations.
//! These flags provide fine-grained control over file system behavior and operations.
//!
//! Note: Not all flags may be applicable or supported by every FUSE implementation.
//! The documentation and usage of these flags are subject to ongoing refinement and validation.
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Copy, Clone)]
    /// Flags used to check file accessibility.
    pub struct AccessMask: i32 {
        /// Check if the file exists.
        const EXISTS = libc::F_OK;
        /// Check if the file is readable.
        const CAN_READ = libc::R_OK;
        /// Check if the file is writable.
        const CAN_WRITE = libc::W_OK;
        /// Check if the file is executable.
        const CAN_EXEC = libc::X_OK;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    /// Flags used in fallocate calls.
    pub struct FallocateFlags: i32 {
        /// Retain file size; don't extend even if offset + len is greater
        #[cfg(target_os = "linux")]
        const KEEP_SIZE = libc::FALLOC_FL_KEEP_SIZE;
        /// Deallocate space (must be ORed with KEEP_SIZE)
        #[cfg(target_os = "linux")]
        const PUNCH_HOLE = libc::FALLOC_FL_PUNCH_HOLE;
        /// Remove a range from the file without leaving a hole
        #[cfg(target_os = "linux")]
        const COLLAPSE_RANGE = libc::FALLOC_FL_COLLAPSE_RANGE;
        /// Zero and ensure allocation of a range
        #[cfg(target_os = "linux")]
        const ZERO_RANGE = libc::FALLOC_FL_ZERO_RANGE;
        /// Insert a hole at the specified range, shifting existing data
        #[cfg(target_os = "linux")]
        const INSERT_RANGE = libc::FALLOC_FL_INSERT_RANGE;
        /// Make shared file data extents private to the file
        #[cfg(target_os = "linux")]
        const UNSHARE_RANGE = libc::FALLOC_FL_UNSHARE_RANGE;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct FUSEAttrFlags: u32 {
        const SUBMOUNT = 1 << 0;
        const DAX = 1 << 1;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct FUSEGetAttrFlags: i32 {
        const GETATTR_FH = 1 << 0;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct FUSEOpenFlags: i32 {
        const KILL_SUIDGID = 1 << 0;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    /// Flags used in the response to a FUSE open operation.
    pub struct FUSEOpenResponseFlags: u32 {
        /// Bypass page cache for this file.
        const DIRECT_IO = 1 << 0;
        /// Keep cached file data after closing.
        const KEEP_CACHE = 1 << 1;
        /// The file is not seekable.
        const NONSEEKABLE = 1 << 2;
        /// Cache directory contents.
        const CACHE_DIR = 1 << 3;
        /// File is a stream (no file position).
        const STREAM = 1 << 4;
        /// Don't flush cached data on close.
        const NOFLUSH = 1 << 5;
        /// Allow parallel direct writes.
        const PARALLEL_DIRECT_WRITES = 1 << 6;
        /// Pass through operations to underlying filesystem.
        const PASSTHROUGH = 1 << 7;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct FUSEIoctlFlags: u32 {
        const COMPAT = 1 << 0;
        const UNRESTRICTED = 1 << 1;
        const RETRY = 1 << 2;
        const IOCTL_32BIT = 1 << 3;
        const DIR = 1 << 4;
        const COMPAT_X32 = 1 << 5;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct FUSEReadFlags: i32 {
        const LOCKOWNER = 1 << 0;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct FUSEReleaseFlags: i32 {
        const FLUSH = 1 << 0;
        const FLOCK_UNLOCK = 1 << 1;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct FUSEFsyncFlags: u32 {
        const FDATASYNC = 1 << 0;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct FUSESetXAttrFlags: i32 {
        const ACL_KILL_SGID = 1 << 0;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct FUSEWriteFlags: u32 {
        const CACHE = 1 << 0;
        const LOCKOWNER = 1 << 1;
        const KILL_SUIDGID = 1 << 2;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct IOCtlFlags: u32 {
        // Placeholder for future flags
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    // c_short in BSD, c_int in linux
    /// Flags representing different types of file locks.
    pub struct LockType: i32 {
        /// No lock held.
        const UNLOCKED = libc::F_UNLCK as i32;
        /// Shared or read lock.
        const READ_LOCK = libc::F_RDLCK as i32;
        /// Exclusive or write lock.
        const WRITE_LOCK = libc::F_WRLCK as i32;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    /// Flags used when opening files.
    pub struct OpenFlags: i32 {
        /// Open for reading only.
        const READ_ONLY = libc::O_RDONLY;
        /// Open for writing only.
        const WRITE_ONLY = libc::O_WRONLY;
        /// Open for reading and writing.
        const READ_WRITE = libc::O_RDWR;
        /// Create file if it doesn't exist.
        const CREATE = libc::O_CREAT;
        /// Fail if file already exists.
        const CREATE_EXCLUSIVE = libc::O_EXCL;
        /// Don't assign controlling terminal.
        const NO_TERMINAL_CONTROL = libc::O_NOCTTY;
        /// Truncate file to zero length.
        const TRUNCATE = libc::O_TRUNC;
        /// Set append mode.
        const APPEND_MODE = libc::O_APPEND;
        /// Use non-blocking mode.
        const NON_BLOCKING_MODE = libc::O_NONBLOCK;
        /// Synchronize data writes.
        const SYNC_DATA_ONLY = libc::O_DSYNC;
        /// Synchronize both data and metadata writes.
        const SYNC_DATA_AND_METADATA = libc::O_SYNC;
        /// Synchronize read operations (Linux only).
        #[cfg(target_os = "linux")]
        const SYNC_READS_AND_WRITES = libc::O_RSYNC;
        /// Fail if not a directory.
        const MUST_BE_DIRECTORY = libc::O_DIRECTORY;
        /// Do not follow symlinks.
        const DO_NOT_FOLLOW_SYMLINKS = libc::O_NOFOLLOW;
        /// Set close-on-exec flag.
        const CLOSE_ON_EXEC = libc::O_CLOEXEC;
        /// Create an unnamed temporary file (Linux only).
        #[cfg(target_os = "linux")]
        const TEMPORARY_FILE = libc::O_TMPFILE;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    /// Flags used in rename operations.
    pub struct RenameFlags: u32 {
        /// Atomically exchange the old and new pathnames. (Linux only)
        #[cfg(target_os = "linux")]
        const EXCHANGE = libc::RENAME_EXCHANGE;
        /// Don't overwrite the destination file if it exists. (Linux only)
        #[cfg(target_os = "linux")]
        const NOREPLACE = libc::RENAME_NOREPLACE;
        const _ = !0;
    }
}
