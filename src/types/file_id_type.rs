use std::{
    ffi::OsString,
    fmt::{Debug, Display},
    path::{Path, PathBuf},
};

use fuser::FileType as FileKind;

use crate::core::InodeResolvable;

use super::arguments::FileAttribute;
use super::inode::*;

/// Represents the type used to identify files in the file system.
///
/// This trait allows different approaches to file identification:
///
/// 1. `Inode`: The user provides their own unique inode numbers.
///    - Pros: Direct control over inode assignment.
///    - Cons: Requires manual management of inode uniqueness.
///    - Root: Represented by the constant ROOT_INODE with a value of 1.
///
/// 2. `PathBuf`: Uses file paths for identification.
///    - Pros: Automatic inode-to-path mapping and caching.
///    - Cons: May have performance overhead for large file systems.
///    - Root: Represented by an empty string. Paths are relative and never begin with a forward slash.
///
/// 3. `Vec<OsString>`: Uses a vector of path components for identification.
///    - Pros: Slightly lower overhead than PathBuf, allows path to be divided into parts.
///    - Cons: Path components are stored in reverse order, which may require additional handling.
///    - Root: Represented by an empty vector.
pub trait FileIdType:
    'static + Debug + Clone + PartialEq + Eq + std::hash::Hash + InodeResolvable
{
    /// Full metadata type for the file system.
    ///
    /// For Inode-based: (Inode, FileAttribute)
    /// - User must provide both Inode and FileAttribute.
    ///
    /// For PathBuf-based: FileAttribute
    /// - User only needs to provide FileAttribute; Inode is managed internally.
    type Metadata;

    /// Minimal metadata type for the file system.
    ///
    /// For Inode-based: (Inode, FileKind)
    /// - User must provide both Inode and FileKind.
    ///
    /// For PathBuf-based: FileKind
    /// - User only needs to provide FileKind; Inode is managed internally.
    type MinimalMetadata;
    #[doc(hidden)]
    type _Id;

    /// Returns a displayable representation of the file identifier.
    ///
    /// This method provides a human-readable string representation of the file identifier,
    /// which can be useful for debugging, logging, or user-facing output.
    fn display(&self) -> impl Display;

    /// Checks if this file identifier represents the root of the filesystem.
    ///
    /// This method determines whether the current file identifier corresponds to the
    /// topmost directory in the filesystem hierarchy.
    fn is_filesystem_root(&self) -> bool;

    #[doc(hidden)]
    fn extract_metadata(metadata: Self::Metadata) -> (Self::_Id, FileAttribute);
    #[doc(hidden)]
    fn extract_minimal_metadata(minimal_metadata: Self::MinimalMetadata) -> (Self::_Id, FileKind);
}

impl FileIdType for Inode {
    type _Id = Inode;
    type Metadata = (Inode, FileAttribute);
    type MinimalMetadata = (Inode, FileKind);

    fn display(&self) -> impl Display {
        format!("{:?}", self)
    }

    fn is_filesystem_root(&self) -> bool {
        *self == ROOT_INODE
    }

    fn extract_metadata(metadata: Self::Metadata) -> (Self::_Id, FileAttribute) {
        metadata
    }

    fn extract_minimal_metadata(minimal_metadata: Self::MinimalMetadata) -> (Self::_Id, FileKind) {
        minimal_metadata
    }
}

impl FileIdType for PathBuf {
    type _Id = ();
    type Metadata = FileAttribute;
    type MinimalMetadata = FileKind;

    fn display(&self) -> impl Display {
        Path::display(self)
    }

    fn is_filesystem_root(&self) -> bool {
        self.as_os_str().is_empty()
    }

    fn extract_metadata(metadata: Self::Metadata) -> (Self::_Id, FileAttribute) {
        ((), metadata)
    }

    fn extract_minimal_metadata(minimal_metadata: Self::MinimalMetadata) -> (Self::_Id, FileKind) {
        ((), minimal_metadata)
    }
}

impl FileIdType for Vec<OsString> {
    type _Id = ();
    type Metadata = FileAttribute;
    type MinimalMetadata = FileKind;

    fn display(&self) -> impl Display {
        // Join all paths with a separator for display
        self.iter()
            .map(|os_str| os_str.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn is_filesystem_root(&self) -> bool {
        self.is_empty()
    }

    fn extract_metadata(metadata: Self::Metadata) -> (Self::_Id, FileAttribute) {
        ((), metadata)
    }

    fn extract_minimal_metadata(minimal_metadata: Self::MinimalMetadata) -> (Self::_Id, FileKind) {
        ((), minimal_metadata)
    }
}
