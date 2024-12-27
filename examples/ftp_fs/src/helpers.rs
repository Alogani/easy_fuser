use std::time::{Duration, SystemTime};

use easy_fuser::prelude::*;
use chrono::prelude::*;


pub fn create_file_attribute(size: u64, modify_time: DateTime<Utc>, is_dir: bool) -> FileAttribute {
    FileAttribute {
        size,
        blocks: (size + 511) / 512,
        atime: SystemTime::UNIX_EPOCH + Duration::from_secs(modify_time.timestamp() as u64),
        mtime: SystemTime::UNIX_EPOCH + Duration::from_secs(modify_time.timestamp() as u64),
        ctime: SystemTime::UNIX_EPOCH + Duration::from_secs(modify_time.timestamp() as u64),
        crtime: SystemTime::UNIX_EPOCH + Duration::from_secs(modify_time.timestamp() as u64),
        kind: if is_dir { FileKind::Directory } else { FileKind::RegularFile },
        perm: 0o755,
        nlink: 1,
        uid: 1000,
        gid: 1000,
        rdev: 0,
        flags: 0,
        blksize: 512,
        ttl: None,
        generation: None,
    }
}

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
        uid: 1000,
        gid: 1000,
        rdev: 0,
        flags: 0,
        blksize: 512,
        ttl: None,
        generation: None,
    }
}