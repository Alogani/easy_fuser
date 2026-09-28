//! File identification types and traits for FUSE filesystems.
//!
//! This module defines the `FileIdType` trait and its implementations, which provide
//! ways to identify files in a FUSE filesystem: `PathBuf`, `MappedInode`, and
//! `Inode`. The older `Vec<OsString>` form remains for compatibility.
//! The module also includes associated types for full and minimal metadata, which
//! are different possible return values in FUSE operations.

use std::{
    ffi::OsString,
    fmt::{Debug, Display},
    hash::Hasher,
    path::{Path, PathBuf},
    sync::{Arc, RwLock, atomic::AtomicU64},
};

use super::arguments::FileAttribute;
use super::inode::*;
use crate::{core::InodeResolvable, inode_mapping::InodeMapper};
use fuser::FileType as FileKind;

/// Represents the type used to identify files in the file system.
///
/// Choose the form that matches how your filesystem finds files:
///
/// 1. `PathBuf`: Receives one path relative to the mount root, without a
///    leading `/` (for example, `dir/file.txt`, not `/dir/file.txt`).
///    easy_fuser assigns the FUSE inode number.
///    - Pros: Simple when your files are naturally addressed by path.
///    - Cons: Two hard-link names are tracked as separate FUSE inodes.
///    - Root: An empty path.
///    - After unlink: If FUSE still refers to an inode, its `PathBuf` is the
///      last remembered path. For example, after unlinking `a.txt`, a later
///      operation can still receive `a.txt`, even though that name no longer
///      exists. It is not replaced with an empty path, and the name could later
///      refer to a different file. Use an open file handle when one is available.
///
/// 2. `Inode`: Receives a FUSE inode number that you assign. Return that number
///    with the metadata from `lookup`, `create`, `link`, and similar operations;
///    later operations receive the same number for that file.
///    - Pros: You control file identity directly, including hard links.
///    - Cons: You must assign unique inode numbers and find the corresponding
///      file yourself.
///    - Root: [`ROOT_INODE`] (1).
///
/// 3. [`MappedInode`]: Receives an inode number assigned by easy_fuser and can
///    ask for its known paths.
///    - Pros: Tracks multiple hard-link names under one inode when a
///      `FuseHandler::link` call succeeds.
///    - Cons: Keeps link records in memory and rebuilds paths when requested.
///      Hard links already present or created outside this filesystem are not
///      discovered automatically.
///    - `paths() -> Vec<PathBuf>`: All currently known paths to the inode,
///      relative to the mount root and without a leading `/`. For example,
///      after linking `a.txt` as `b.txt`, the result contains both.
///    - `parts_paths() -> Vec<Vec<OsString>>`: The same paths represented as
///      OS-native components in root-to-file order. A path `dir/file.txt` is
///      represented as `["dir", "file.txt"]` within the outer list. This is
///      faster when you can use components directly because it skips building
///      `PathBuf`s.
///    - `inode() -> Inode`: The assigned FUSE inode number.
///    - Root: [`ROOT_INODE`] (1), with an empty path.
///
///    `paths()` and `parts_paths()` each reconstruct a new result when called;
///    neither caches it. Keep the returned value if you need it again during
///    the same operation, and call again after links or names change. After
///    the last name is unlinked, both return an empty list while the inode can
///    still be in use by an open file.
///
/// 4. `Vec<OsString>`: The older path-component form, deprecated since 0.7.0.
///    Components are ordered from file back toward the root.
pub trait FileIdType:
    'static + Debug + Clone + PartialEq + Eq + Send + std::hash::Hash + InodeResolvable
{
    /// Full metadata type for the file system.
    ///
    /// For Inode-based: (Inode, FileAttribute)
    /// - User must provide both Inode and FileAttribute.
    ///
    /// For `PathBuf` and `MappedInode`: FileAttribute
    /// - User only needs to provide FileAttribute; Inode is managed internally.
    type Metadata: Send;

    /// Minimal metadata type for the file system.
    ///
    /// For Inode-based: (Inode, FileKind)
    /// - User must provide both Inode and FileKind.
    ///
    /// For `PathBuf` and `MappedInode`: FileKind
    /// - User only needs to provide FileKind; Inode is managed internally.
    type MinimalMetadata: Send;
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

#[allow(useless_deprecated)]
#[deprecated(since = "0.7.0", note = "please use `MappedInode` instead")]
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

/// An inode assigned by easy_fuser that can resolve its currently known paths.
///
/// `paths()` can be empty if the final name was unlinked while FUSE still
/// holds the inode. A handler serving a pathless open file can use its open
/// file handle. This type is created by the crate, not by a handler. Paths are
/// reconstructed when requested and are not cached in this value.
#[derive(Clone)]
pub struct MappedInode {
    inode: Inode,
    mapper: Arc<RwLock<InodeMapper<AtomicU64>>>,
}

impl MappedInode {
    pub(crate) fn new(inode: Inode, mapper: Arc<RwLock<InodeMapper<AtomicU64>>>) -> Self {
        Self { inode, mapper }
    }

    /// The FUSE inode number shared by all registered hard-link names.
    pub fn inode(&self) -> Inode {
        self.inode
    }

    /// Every currently known path, relative to the mounted filesystem root.
    /// Paths have no leading `/`; the root is an empty path.
    ///
    /// Reconstructs the paths when called and returns an owned snapshot. It
    /// converts the OS-native components returned by `parts_paths()` into
    /// `PathBuf`s. Keep the result if you need to read it more than once during
    /// an operation; call this again after links or names change.
    pub fn paths(&self) -> Vec<PathBuf> {
        self.parts_paths()
            .into_iter()
            .map(|parts| parts.iter().collect())
            .collect()
    }

    /// The same paths as `paths()`, split into OS-native components.
    ///
    /// This avoids constructing `PathBuf`s, so it is faster than `paths()`
    /// when you can use components directly. It still reconstructs every path
    /// from the mapper's links and returns an owned snapshot on each call; it
    /// does not borrow the stored links.
    pub fn parts_paths(&self) -> Vec<Vec<OsString>> {
        self.mapper
            .read()
            .expect("Failed to acquire read lock")
            .resolve(&self.inode)
            .unwrap_or_default()
    }
}

impl PartialEq for MappedInode {
    fn eq(&self, other: &Self) -> bool {
        self.inode == other.inode && Arc::ptr_eq(&self.mapper, &other.mapper)
    }
}
impl Eq for MappedInode {}
impl std::hash::Hash for MappedInode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inode.hash(state);
        Arc::as_ptr(&self.mapper).hash(state);
    }
}
impl Debug for MappedInode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MappedInode")
            .field("inode", &self.inode)
            .field("paths", &self.paths())
            .finish()
    }
}
impl FileIdType for MappedInode {
    type _Id = ();
    type Metadata = FileAttribute;
    type MinimalMetadata = FileKind;

    fn display(&self) -> impl Display {
        format!("{:?}", self)
    }

    fn is_filesystem_root(&self) -> bool {
        self.inode == ROOT_INODE
    }

    fn extract_metadata(metadata: Self::Metadata) -> (Self::_Id, FileAttribute) {
        ((), metadata)
    }

    fn extract_minimal_metadata(minimal_metadata: Self::MinimalMetadata) -> (Self::_Id, FileKind) {
        ((), minimal_metadata)
    }
}
