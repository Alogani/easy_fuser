pub use bsd_fs_like::*;

use std::{ffi::c_void, path::Path};

use crate::PosixError;
use libc::{self, c_char, c_int, c_uint, off_t, size_t, ssize_t};

use super::{cstring_from_path, FileDescriptor, StatFs};

pub(super) unsafe fn setxattr(
    path: *const c_char,
    name: *const c_char,
    value: *const c_void,
    _size: size_t,
    _flags: c_int,
) -> c_int {
    libc::extattr_set_file(path, libc::EXTATTR_NAMESPACE_USER, name, value, value.len())
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
