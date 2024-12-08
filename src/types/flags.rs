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
