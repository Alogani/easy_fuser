mod arguments;
mod errors;
mod file_descriptor;
mod file_id_type;
mod flags;
mod thread_mode;

pub use self::{arguments::*, errors::*, file_descriptor::*, file_id_type::FileIdType, flags::*};

pub use fuser::{FileType as FileKind, KernelConfig, TimeOrNow};

pub mod private {
    pub use super::file_id_type::*;
    pub use super::thread_mode::*;

    use std::time::Duration;

    use super::*;
    use fuser::FileAttr as FuseFileAttr;

    /// FuseFileAttr, Option<ttl>, Option<generation>
    pub type FuseMetaData = (FuseFileAttr, Option<Duration>, Option<u64>);

    impl FileAttribute {
        pub fn to_fuse(self, ino: u64) -> FuseMetaData {
            (
                FuseFileAttr {
                    ino,
                    size: self.size,
                    blocks: self.blocks,
                    atime: self.atime,
                    mtime: self.mtime,
                    ctime: self.ctime,
                    crtime: self.crtime,
                    kind: self.kind,
                    perm: self.perm,
                    nlink: self.nlink,
                    uid: self.uid,
                    gid: self.gid,
                    rdev: self.rdev,
                    blksize: self.blksize,
                    flags: self.flags,
                },
                self.ttl,
                self.generation,
            )
        }
    }
}
