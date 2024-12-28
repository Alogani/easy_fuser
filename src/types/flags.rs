use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct AccessMask: i32 {
        const EXISTS = libc::F_OK;
        const CAN_READ = libc::R_OK;
        const CAN_WRITE = libc::W_OK;
        const CAN_EXEC = libc::X_OK;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct FallocateFlags: i32 {
        const KEEP_SIZE = libc::FALLOC_FL_KEEP_SIZE; // Retain file size; don't extend even if offset + len is greater
        const PUNCH_HOLE = libc::FALLOC_FL_PUNCH_HOLE; // Deallocate space (must be ORed with KEEP_SIZE)
        const COLLAPSE_RANGE = libc::FALLOC_FL_COLLAPSE_RANGE; // Remove a range from the file without leaving a hole
        const ZERO_RANGE = libc::FALLOC_FL_ZERO_RANGE; // Zero and ensure allocation of a range
        const INSERT_RANGE = libc::FALLOC_FL_INSERT_RANGE; // Insert a hole at the specified range, shifting existing data
        const UNSHARE_RANGE = libc::FALLOC_FL_UNSHARE_RANGE; // Make shared file data extents private to the file
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
    pub struct FUSEOpenResponseFlags: u32 {
        const DIRECT_IO = 1 << 0;
        const KEEP_CACHE = 1 << 1;
        const NONSEEKABLE = 1 << 2;
        const CACHE_DIR = 1 << 3;
        const STREAM = 1 << 4;
        const NOFLUSH = 1 << 5;
        const PARALLEL_DIRECT_WRITES = 1 << 6;
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
    pub struct LockType: i32 {
        // c_short in BSD, c_int in linux
        const UNLOCKED = libc::F_UNLCK as i32;
        const READ_LOCK = libc::F_RDLCK as i32;
        const WRITE_LOCK = libc::F_WRLCK as i32;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct OpenFlags: i32 {
        const READ_ONLY = libc::O_RDONLY;
        const WRITE_ONLY = libc::O_WRONLY;
        const READ_WRITE = libc::O_RDWR;
        const CREATE = libc::O_CREAT;
        const CREATE_EXCLUSIVE = libc::O_EXCL;
        const NO_TERMINAL_CONTROL = libc::O_NOCTTY;
        const TRUNCATE = libc::O_TRUNC;
        const APPEND_MODE = libc::O_APPEND;
        const NON_BLOCKING_MODE = libc::O_NONBLOCK;
        const SYNC_DATA_ONLY = libc::O_DSYNC;
        const SYNC_DATA_AND_METADATA = libc::O_SYNC;
        #[cfg(target_os = "linux")]
        const SYNC_READS_AND_WRITES = libc::O_RSYNC;
        const MUST_BE_DIRECTORY = libc::O_DIRECTORY;
        const DO_NOT_FOLLOW_SYMLINKS = libc::O_NOFOLLOW;
        const CLOSE_ON_EXEC = libc::O_CLOEXEC;
        #[cfg(target_os = "linux")]
        const TEMPORARY_FILE = libc::O_TMPFILE;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct RenameFlags: u32 {
        #[cfg(target_os = "linux")]
        const EXCHANGE = libc::RENAME_EXCHANGE;
        #[cfg(target_os = "linux")]
        const NOREPLACE = libc::RENAME_NOREPLACE;
        const _ = !0;
    }
}