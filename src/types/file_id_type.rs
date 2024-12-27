use std::{
    ffi::OsString,
    fmt::{Debug, Display},
    path::{Path, PathBuf},
};

use fuser::FileType as FileKind;

use crate::core::GetConverter;

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
    'static + GetConverter + Debug + Clone + PartialEq + Eq + std::hash::Hash
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
    type _Id;

    fn display(&self) -> impl Display;

    fn is_fuse_root(&self) -> bool;

    fn extract_metadata(metadata: Self::Metadata) -> (Self::_Id, FileAttribute);
    fn extract_minimal_metadata(minimal_metadata: Self::MinimalMetadata) -> (Self::_Id, FileKind);
}

impl FileIdType for Inode {
    type _Id = Inode;
    type Metadata = (Inode, FileAttribute);
    type MinimalMetadata = (Inode, FileKind);

    fn display(&self) -> impl Display {
        format!("{:?}", self)
    }

    fn is_fuse_root(&self) -> bool {
        *self == ROOT_INODE
    }

    /// For internal usage
    fn extract_metadata(metadata: Self::Metadata) -> (Self::_Id, FileAttribute) {
        metadata
    }

    /// For internal usage
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

    fn is_fuse_root(&self) -> bool {
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

    fn is_fuse_root(&self) -> bool {
        self.is_empty()
    }

    fn extract_metadata(metadata: Self::Metadata) -> (Self::_Id, FileAttribute) {
        ((), metadata)
    }

    fn extract_minimal_metadata(minimal_metadata: Self::MinimalMetadata) -> (Self::_Id, FileKind) {
        ((), minimal_metadata)
    }
}

/// Usage:
/// ```text
/// fn test<TId: FileIdType>(metadata: TId::Metadata) -> FileAttribute
/// {
///     let (_a, b) = unpack_metadata::<TId>(metadata);
///     b
/// }
/// ```
pub fn unpack_metadata<TId>(metadata: TId::Metadata) -> (<TId as FileIdType>::_Id, FileAttribute)
where
    TId: FileIdType,
{
    TId::extract_metadata(metadata)
}

pub fn unpack_minimal_metadata<TId>(
    minimal_metadata: TId::MinimalMetadata,
) -> (<TId as FileIdType>::_Id, FileKind)
where
    TId: FileIdType,
{
    TId::extract_minimal_metadata(minimal_metadata)
}
