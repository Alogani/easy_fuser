//! Types and structures for FUSE filesystem operations.
//!
//! This module provides various type definitions and structures that are used
//! throughout the `easy_fuser` crate to represent FUSE-related concepts and data.
//!
//! # Modules
//!
//! - \[arguments\]: Defines argument types and structures for FUSE operations.
//! - \[errors\]: Contains error types and handling for FUSE operations.
//! - \[file_descriptor\]: Provides types related to file descriptors.
//! - \[file_id_type\]: Defines traits for file identification.
//! - \[flags\]: Contains flag definitions for various FUSE operations.
//! - \[inode\]: Defines the `Inode` type for representing filesystem objects.
//!
//! # Re-exports
//!
//! This module re-exports key types from its submodules for easier access, as well as
//! some types from the `fuser` crate that are commonly used in FUSE operations.

pub mod arguments;
pub mod errors;
pub mod file_handle;
mod file_id_type;
pub mod flags;
mod inode;
pub mod mount_threads;

pub use self::{
    arguments::*, errors::*, file_handle::*, file_id_type::*, flags::*, inode::*, mount_threads::*,
};

pub use fuser::{FileType as FileKind, KernelConfig, TimeOrNow};

/// Checks whether a request has the requested access to a file attribute.
///
/// Permissions are selected from the owner, primary-group, or other mode bits.
/// Requests from uid 0 are allowed without checking the mode bits. For
/// directories, execute permission is required for access, and write requests
/// also require the directory's write bit.
///
/// [`RequestInfo`] contains only the request's primary group ID, so this check
/// does not account for supplementary groups. It is a userspace policy helper
/// and does not replace kernel-side permission enforcement.
pub fn check_mode_access(
    req: &RequestInfo,
    attr: &FileAttribute,
    mask: AccessFlags,
) -> FuseResult<()> {
    if req.uid == 0 {
        return Ok(());
    }

    let (read_bit, write_bit, execute_bit) = if req.uid == attr.uid {
        (0o400, 0o200, 0o100)
    } else if req.gid == attr.gid {
        (0o040, 0o020, 0o010)
    } else {
        (0o004, 0o002, 0o001)
    };

    let mut allowed_mask = AccessFlags::empty();
    if attr.perm & read_bit != 0 {
        allowed_mask |= AccessFlags::R_OK;
    }
    if attr.perm & write_bit != 0 {
        allowed_mask |= AccessFlags::W_OK;
    }
    if attr.perm & execute_bit != 0 {
        allowed_mask |= AccessFlags::X_OK;
    }

    if attr.kind == FileKind::Directory {
        if !allowed_mask.contains(AccessFlags::X_OK) {
            return Err(
                ErrorKind::PermissionDenied.to_error("Execute permission required for directory")
            );
        }
        if mask.contains(AccessFlags::W_OK) && !allowed_mask.contains(AccessFlags::W_OK) {
            return Err(ErrorKind::PermissionDenied
                .to_error("Write permission required for directory modification"));
        }
    }

    if allowed_mask.contains(mask) {
        Ok(())
    } else {
        Err(ErrorKind::PermissionDenied.to_error("Permission denied"))
    }
}
