use std::{fmt::Display, path::{Path, PathBuf}};

use fuser::FileType as FileKind;

use super::arguments::*;

/// FileIdType can have two values:
/// - Inode: in which case the user shall provide its own unique inode (at least a valid one)
/// - PathBuf: in which the inode to path mapping will be done and cached automatically
pub trait FileIdType: Send + std::fmt::Debug + 'static {
    type Id;
    type Metadata: MetadataExt<FileIdType = Self>;
    type MinimalMetadata: MinimalMetadataExt<FileIdType = Self>;

    fn display(&self) -> impl Display;
}

impl FileIdType for Inode {
    type Id = Inode;
    type Metadata = (Inode, FileAttribute);
    type MinimalMetadata = (Inode, FileKind);


    fn display(&self) -> impl Display {
        format!("{:?}", self)
    }
}
impl FileIdType for PathBuf {
    type Id = ();
    type Metadata = FileAttribute;
    type MinimalMetadata = FileKind;

    fn display(&self) -> impl Display {
        Path::display(self)
    }
}

pub trait MetadataExt {
    type FileIdType: FileIdType;

    fn extract_metadata(metadata: Self) -> (<Self::FileIdType as FileIdType>::Id, FileAttribute);
}

pub trait MinimalMetadataExt {
    type FileIdType: FileIdType;
    fn extract_minimal(minimal_metadata: Self) -> (<Self::FileIdType as FileIdType>::Id, FileKind);
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
pub fn unpack_metadata<T>(metadata: T::Metadata) -> (<T as FileIdType>::Id, FileAttribute)
where
    T: FileIdType,
    T::Metadata: MetadataExt<FileIdType=T>,
{
    T::Metadata::extract_metadata(metadata)
}


pub fn unpack_minimal_metadata<T>(minimal_metadata: T::MinimalMetadata) -> (<T as FileIdType>::Id, FileKind)
where
    T: FileIdType,
    T::MinimalMetadata: MinimalMetadataExt<FileIdType=T>,
{
    T::MinimalMetadata::extract_minimal(minimal_metadata)
}
