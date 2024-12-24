use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use fuser::FileType as FileKind;

use super::arguments::*;

/// FileIdType can have two values:
/// - Inode: in which case the user shall provide its own unique inode (at least a valid one)
/// - PathBuf: in which the inode to path mapping will be done and cached automatically
pub trait FileIdType: Send + std::fmt::Debug + Clone + 'static {
    type Metadata: MetadataExt<FileIdType = Self>;
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

pub trait MetadataExt: Send + Sync {
    type FileIdType: FileIdType;

    fn extract_metadata(metadata: Self) -> (<Self::FileIdType as FileIdType>::_Id, FileAttribute);
}

pub trait MinimalMetadataExt: Send + Sync {
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
/// ```
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
