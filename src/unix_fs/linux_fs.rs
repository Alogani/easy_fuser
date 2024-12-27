use std::{ffi::c_void, path::Path};

use crate::PosixError;
use libc::{self, c_char, c_int, c_uint, off_t, size_t, ssize_t};

use super::{cstring_from_path, FileDescriptor, StatFs};

pub(crate) fn get_errno() -> i32 {
    unsafe { *libc::__errno_location() }
}

pub(super) fn set_errno(errno: i32) {
    unsafe { *libc::__errno_location() = errno };
}

pub(super) unsafe fn renameat2(
    olddirfd: c_int,
    oldpath: *const c_char,
    newdirfd: c_int,
    newpath: *const c_char,
    flags: c_uint,
) -> c_int {
    libc::renameat2(olddirfd, oldpath, newdirfd, newpath, flags)
}

pub(super) unsafe fn fdatasync(fd: c_int) -> c_int {
    libc::fdatasync(fd)
}

pub(super) unsafe fn fallocate(fd: c_int, mode: c_int, offset: off_t, len: off_t) -> c_int {
    libc::fallocate(fd, mode, offset, len)
}

pub(super) unsafe fn setxattr(
    path: *const c_char,
    name: *const c_char,
    value: *const c_void,
    size: size_t,
    _position: u32,
    flags: c_int,
) -> c_int {
    libc::setxattr(path, name, value, size, flags)
}

pub(super) unsafe fn getxattr(
    path: *const c_char,
    name: *const c_char,
    value: *mut c_void,
    size: size_t,
) -> ssize_t {
    libc::getxattr(path, name, value, size)
}

pub(super) unsafe fn listxattr(path: *const c_char, list: *mut c_char, size: size_t) -> ssize_t {
    libc::listxattr(path, list, size)
}

pub(super) unsafe fn removexattr(path: *const c_char, name: *const c_char) -> c_int {
    libc::removexattr(path, name)
}

/// Retrieves file system statistics for the specified path.
///
/// This function is equivalent to the FUSE `statfs` operation.
pub fn statfs(path: &Path) -> Result<StatFs, PosixError> {
    let c_path = cstring_from_path(path)?;
    let mut stat: libc::statvfs64 = unsafe { std::mem::zeroed() };

    // Use statvfs64 to get file system stats
    let result = unsafe { libc::statvfs64(c_path.as_ptr(), &mut stat) };
    if result != 0 {
        return Err(PosixError::last_error(format!(
            "{}: statfs failed",
            path.display()
        )));
    }

    Ok(StatFs {
        total_blocks: stat.f_blocks as u64,
        free_blocks: stat.f_bfree as u64,
        available_blocks: stat.f_bavail as u64,
        total_files: stat.f_files as u64,
        free_files: stat.f_ffree as u64,
        block_size: stat.f_bsize as u32,
        max_filename_length: stat.f_namemax as u32,
        fragment_size: stat.f_frsize as u32,
    })
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
    let result = unsafe {
        libc::copy_file_range(
            fd_in.clone().into(),
            offset_in as *mut libc::off_t,
            fd_out.clone().into(),
            offset_out as *mut libc::off_t,
            len as usize,
            0, // placeholder
        )
    };
    if result == -1 {
        return Err(PosixError::last_error("copyfilerange failed"));
    }
    Ok(result as u32)
}
