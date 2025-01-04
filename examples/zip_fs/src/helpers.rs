use std::time::UNIX_EPOCH;

use easy_fuser::prelude::*;
use zip::read::ZipFile;

pub fn get_root_attribute() -> FileAttribute {
    FileAttribute {
        size: 0,
        blocks: 0,
        atime: std::time::UNIX_EPOCH,
        mtime: std::time::UNIX_EPOCH,
        ctime: std::time::UNIX_EPOCH,
        crtime: std::time::UNIX_EPOCH,
        kind: FileKind::Directory,
        perm: 0o755,
        nlink: 2,
        uid: 0,
        gid: 0,
        rdev: 0,
        flags: 0,
        blksize: 512,
        ttl: None,
        generation: None,
    }
}

pub fn create_file_attribute(file: &ZipFile, is_dir: bool) -> FileAttribute {
    FileAttribute {
        size: file.size(),
        blocks: (file.size() + 511) / 512,
        atime: UNIX_EPOCH,
        mtime: UNIX_EPOCH,
        ctime: UNIX_EPOCH,
        crtime: UNIX_EPOCH,
        kind: if is_dir {
            FileKind::Directory
        } else {
            FileKind::RegularFile
        },
        perm: 0o444,
        nlink: 1,
        uid: 0,
        gid: 0,
        rdev: 0,
        flags: 0,
        blksize: 512,
        ttl: None,
        generation: None,
    }
}
