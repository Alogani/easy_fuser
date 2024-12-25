/// Represents an inode number in a FUSE (Filesystem in Userspace) filesystem.
///
/// `Inode` implements the `FileIdType` trait, which is used as a generic parameter
/// throughout this crate. This implementation allows `Inode` to be used as a file
/// identifier in various fuse operations.
///
/// For more detailed information about file identification and the `FileIdType` trait,
/// please refer to the documentation of the `FileIdType` trait.
///
/// In FUSE filesystems, inode numbers are unique identifiers for file system objects,
/// distinct from traditional Unix-style inodes. The user of this library is responsible
/// for ensuring the uniqueness of these numbers. Inodes are created for each function
/// that returns an Inode in FuseHandler, and dropped via the forget function of FuseHandler.
/// The lookup function is a special case that increments an internal count (handled inside
/// libfuse) and can create a new inode if it doesn't exist.
///
/// Note: This concept is separate from the traditional Unix inode, which is a data structure
/// describing file system objects like files or directories.
///
/// This struct is a wrapper around a u64, providing type safety and
/// semantic meaning to inode numbers in the FUSE filesystem implementation.
///
/// For users who prefer not to manage inodes directly, this library also supports
/// using `PathBuf` as `FileIdType`. This alternative approach allows for file
/// identification based on paths rather than inode numbers.

pub const ROOT_INODE: Inode = Inode(1);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Inode(u64);

impl From<u64> for Inode {
    /// Converts a u64 into an Inode.
    ///
    /// This allows for easy creation of Inode instances from raw inode numbers.
    fn from(value: u64) -> Self {
        Inode(value)
    }
}

impl From<Inode> for u64 {
    // Converts a u64 into an Inode.
    ///
    /// This allows for easy creation of Inode instances from raw inode numbers.
    fn from(value: Inode) -> Self {
        value.0
    }
}
