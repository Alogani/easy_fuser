pub use super::bsd_like_fs::*;

use super::{cstring_from_path, StatFs};
use crate::PosixError;

use std::ffi::c_void;
use libc::{self, c_char, c_int, off_t, size_t, ssize_t};

use super::{cstring_from_path, StatFs};
use std::path::Path;

pub(super) unsafe fn fallocate(fd: c_int, _mode: c_int, offset: off_t, len: off_t) -> c_int {
    libc::posix_fallocate(fd, offset, len)
}

pub(super) unsafe fn setxattr(
    path: *const c_char,
    name: *const c_char,
    value: *const c_void,
    size: size_t,
    _position: u32,
    _flags: c_int,
) -> c_int {
    libc::extattr_set_file(path, libc::EXTATTR_NAMESPACE_USER, name, value, size).try_into().unwrap()
}

pub(super) unsafe fn getxattr(
    path: *const c_char,
    name: *const c_char,
    value: *mut c_void,
    size: size_t,
) -> ssize_t {
    libc::extattr_get_file(path, libc::EXTATTR_NAMESPACE_USER, name, value, size)
}

pub(super) unsafe fn listxattr(path: *const c_char, list: *mut c_char, size: size_t) -> ssize_t {
    libc::extattr_list_file(
        path,
        libc::EXTATTR_NAMESPACE_USER,
        list as *mut c_void,
        size,
    )
}

pub(super) unsafe fn removexattr(path: *const c_char, name: *const c_char) -> c_int {
    libc::extattr_delete_file(path, libc::EXTATTR_NAMESPACE_USER, name)
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
        max_filename_length: stat.f_namemax as u32,
        fragment_size: stat.f_bsize as u32, // BSD doesn't have f_frsize, so we use f_bsize
    })
}
