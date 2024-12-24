use std::{
    fmt::{Debug, Display},
    path::{Path, PathBuf},
};

use fuser::FileType as FileKind;

use crate::core::GetConverter;

use super::arguments::FileAttribute;
use super::inode::*;

/// Represents the type used to identify files in the file system.
///
/// This trait allows for two different approaches to file identification:
///
/// 1. Inode-based: The user provides their own unique inode numbers.
///    - Pros: Direct control over inode assignment.
///    - Cons: Requires manual management of inode uniqueness.
///
/// 2. PathBuf-based: Uses file paths for identification.
///    - Pros: Automatic inode-to-path mapping and caching.
///    - Cons: May have performance overhead for large file systems.
///
pub trait FileIdType: GetConverter + Debug + Clone + 'static {
    /// Full metadata type for the file system.
    ///
    /// For Inode-based: (Inode, FileAttribute)
    /// - User must provide both Inode and FileAttribute.
    ///
    /// For PathBuf-based: FileAttribute
    /// - User only needs to provide FileAttribute; Inode is managed internally.
    type Metadata: MetadataExt<FileIdType = Self>;

    /// Minimal metadata type for the file system.
    ///
    /// For Inode-based: (Inode, FileKind)
    /// - User must provide both Inode and FileKind.
    ///
    /// For PathBuf-based: FileKind
    /// - User only needs to provide FileKind; Inode is managed internally.
    type MinimalMetadata: MinimalMetadataExt<FileIdType = Self>;
    type _Id;

    fn display(&self) -> impl Display;
}

impl FileIdType for Inode {
    type _Id = Inode;
    type Metadata = (Inode, FileAttribute);
    type MinimalMetadata = (Inode, FileKind);

    fn display(&self) -> impl Display {
        format!("{:?}", self)
    }
}
impl FileIdType for PathBuf {
    type _Id = ();
    type Metadata = FileAttribute;
    type MinimalMetadata = FileKind;

    fn display(&self) -> impl Display {
        Path::display(self)
    }
}

pub trait MetadataExt {
    type FileIdType: FileIdType;

    fn extract_metadata(metadata: Self) -> (<Self::FileIdType as FileIdType>::_Id, FileAttribute);
}

pub trait MinimalMetadataExt {
    type FileIdType: FileIdType;
    fn extract_minimal(minimal_metadata: Self)
        -> (<Self::FileIdType as FileIdType>::_Id, FileKind);
}

impl MetadataExt for (Inode, FileAttribute) {
    type FileIdType = Inode;

    fn extract_metadata(metadata: Self) -> (Inode, FileAttribute) {
        (metadata.0, metadata.1)
    }
}

impl MinimalMetadataExt for (Inode, FileKind) {
    type FileIdType = Inode;

    fn extract_minimal(minimal_metadata: Self) -> (Inode, FileKind) {
        (minimal_metadata.0, minimal_metadata.1)
    }
}

impl MetadataExt for FileAttribute {
    type FileIdType = PathBuf;

    fn extract_metadata(metadata: Self) -> ((), FileAttribute) {
        ((), metadata)
    }
}

impl MinimalMetadataExt for FileKind {
    type FileIdType = PathBuf;

    fn extract_minimal(minimal_metadata: Self) -> ((), FileKind) {
        // Assuming there is some conversion logic from FileKind to FileKind
        ((), minimal_metadata)
    }
}

/// Usage:
/// ```text
/// fn test<T: FileIdType>(metadata: T::Metadata) -> FileAttribute
/// {
///     let (_a, b) = unpack_metadata::<T>(metadata);
///     b
/// }
/// ```
pub fn unpack_metadata<T>(metadata: T::Metadata) -> (<T as FileIdType>::_Id, FileAttribute)
where
    T: FileIdType,
    T::Metadata: MetadataExt<FileIdType = T>,
{
    T::Metadata::extract_metadata(metadata)
}

pub fn unpack_minimal_metadata<T>(
    minimal_metadata: T::MinimalMetadata,
) -> (<T as FileIdType>::_Id, FileKind)
where
    T: FileIdType,
    T::MinimalMetadata: MinimalMetadataExt<FileIdType = T>,
{
    T::MinimalMetadata::extract_minimal(minimal_metadata)
}

/*
pub enum ExtractedMetadata<T: FileIdType> {
    Metadata(T::_Id, FileAttribute),
    MinimalMetadata(T::_Id, FileKind),
}

impl<T: FileIdType> ExtractedMetadata<T> {
    pub fn unpack_metadata(self) -> (T::_Id, FileAttribute) {
        match self {
            ExtractedMetadata::Metadata(id, attr) => (id, attr),
            _ => panic!("")
        }
    }

    pub fn unpack_minimal_metadata(self) -> (T::_Id, FileKind) {
        match self {
            ExtractedMetadata::MinimalMetadata(id, kind) => (id, kind),
            _ => panic!("")
        }
    }
}

impl ExtractedMetadata<Inode> {
    pub fn from_metadata(metadata: <Inode as FileIdType>::Metadata) -> Self
    {
        ExtractedMetadata::Metadata(metadata.0, metadata.1)
    }

    pub fn from_minimal_metadata(minimal_metadata: <Inode as FileIdType>::MinimalMetadata) -> Self
    {
        ExtractedMetadata::MinimalMetadata(minimal_metadata.0, minimal_metadata.1)
    }
}

impl ExtractedMetadata<PathBuf> {
    pub fn from_metadata(metadata: <PathBuf as FileIdType>::Metadata) -> Self
    {
        ExtractedMetadata::Metadata((), metadata)
    }

    pub fn from_minimal_metadata(minimal_metadata: <PathBuf as FileIdType>::MinimalMetadata) -> Self
    {
        ExtractedMetadata::MinimalMetadata((), minimal_metadata)
    }
}

use crate::fuse_handler::FuseHandler;
use std::ffi::OsStr;
fn test<T: FileIdType, U: FuseHandler<T>>(handler: U, req: &RequestInfo, parent_id: T, name: &OsStr) {
    let metadata = handler.lookup(req, parent_id, name);
    let (id, attr) = ExtractedMetadata::from_metadata::<T>(metadata).unpack_metadata();
}
*/
