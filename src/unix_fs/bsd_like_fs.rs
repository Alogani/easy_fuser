use libc::{self, c_char, c_int, c_uint, off_t};

use super::{cstring_from_path, FileDescriptor};

use crate::{ErrorKind, PosixError};

pub(super) fn get_errno() -> i32 {
    unsafe { *libc::__error()() }
}

pub(super) fn set_errno(errno: i32) {
    unsafe { *libc::__error()() = errno };
}

// Flags are ignored
pub(super) unsafe fn renameat2(
    olddirfd: c_int,
    oldpath: *const c_char,
    newdirfd: c_int,
    newpath: *const c_char,
    _flags: c_uint,
) -> c_int {
    libc::renameat(olddirfd, oldpath, newdirfd, newpath)
}

pub(super) unsafe fn fdatasync(fd: c_int) -> c_int {
    libc::fsync(fd)
}

/// Copies a range of data from one file to another.
///
/// This function is equivalent to the FUSE `copy_file_range` operation.
///
/// It copies `len` bytes from the file descriptor `fd_in` starting at offset `offset_in`
/// to the file descriptor `fd_out` starting at offset `offset_out`. The function returns
/// the number of bytes actually copied, which may be less than requested.
///
/// Note: This function is not available on all platforms, like BSD, in that case, it will return not implemented.
pub fn copy_file_range(
    fd_in: &FileDescriptor,
    offset_in: i64,
    fd_out: &FileDescriptor,
    offset_out: i64,
    len: u64,
) -> Result<u32, PosixError> {
    Err(PosixError::new(
        ErrorKind::FunctionNotImplemented,
        "copy_file_range is not implemented on this platform" as &str,
    )
    .into())
}
