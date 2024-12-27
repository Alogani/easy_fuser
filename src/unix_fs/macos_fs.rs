pub use bsd_fs_like::*;

use std::{ffi::c_void, path::Path};

use crate::PosixError;
use libc::{self, c_char, c_int, c_uint, off_t, size_t, ssize_t};

use super::{cstring_from_path, FileDescriptor, StatFs};

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
