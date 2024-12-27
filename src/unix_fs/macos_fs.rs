pub use super::bsd_like_fs::*;

use std::ffi::c_void;

use crate::PosixError;
use libc::{self, c_char, c_int, size_t, ssize_t};
use super::{cstring_from_path, StatFs};

pub(super) unsafe fn setxattr(
    path: *const c_char,
    name: *const c_char,
    value: *const c_void,
    size: size_t,
    position: u32,
    flags: c_int,
) -> c_int {
    libc::setxattr(path, name, value, size, position, flags)
}

pub(super) unsafe fn getxattr(
    path: *const c_char,
    name: *const c_char,
    value: *mut c_void,
    size: size_t,
) -> ssize_t {
    libc::getxattr(path, name, value, size, 0, 0)
}

pub(super) unsafe fn listxattr(path: *const c_char, list: *mut c_char, size: size_t) -> ssize_t {
    libc::listxattr(path, list, size, 0)
}

pub(super) unsafe fn removexattr(path: *const c_char, name: *const c_char) -> c_int {
    libc::removexattr(path, name, 0)
}

/// Retrieves file system statistics for the specified path.
///
/// This function is equivalent to the FUSE `statfs` operation.
pub fn statfs(path: &Path) -> Result<StatFs, PosixError> {
    let c_path = cstring_from_path(path)?;
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };

    // Use statfs to get file system stats
    let result = unsafe { libc::statfs(c_path.as_ptr(), &mut stat) };
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
        max_filename_length: 255,
        fragment_size: stat.f_bsize as u32, // BSD doesn't have f_frsize, so we use f_bsize
    })
}