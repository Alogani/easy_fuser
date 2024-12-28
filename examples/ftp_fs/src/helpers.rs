use std::{path::Path, time::{Duration, SystemTime}};
use easy_fuser::prelude::*;
use chrono::NaiveDateTime;
use log::error;
use suppaftp::FtpStream;

use crate::DirectoryDetectionMethod;


pub fn create_file_attribute(size: u64, modify_time: NaiveDateTime, is_dir: bool) -> FileAttribute {
    FileAttribute {
        size,
        blocks: (size.saturating_add(511)) / 512,
        atime: SystemTime::UNIX_EPOCH + Duration::from_secs(modify_time.and_utc().timestamp() as u64),
        mtime: SystemTime::UNIX_EPOCH + Duration::from_secs(modify_time.and_utc().timestamp() as u64),
        ctime: SystemTime::UNIX_EPOCH + Duration::from_secs(modify_time.and_utc().timestamp() as u64),
        crtime: SystemTime::UNIX_EPOCH + Duration::from_secs(modify_time.and_utc().timestamp() as u64),
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

pub fn get_file_attribute(ftp: &mut FtpStream, path: &Path, detection_method: &DirectoryDetectionMethod) -> Option<FileAttribute> {
    let pathname = path.to_str().unwrap_or_default();

    let is_dir = is_directory(ftp, pathname, detection_method);
    
    let size = if is_dir {
        0 // Directories typically have a size of 0
    } else {
        match ftp.size(pathname) {
            Ok(s) => s as u64,
            Err(e) => {
                error!("Error getting size for {}: {:?}", pathname, e);
                u32::MAX as u64
            }
        }
    };
    println!("size: {:?}", size);

    let modify_time = match ftp.mdtm(pathname) {
        Ok(t) => t,
        Err(e) => {
            error!("Error getting modify time for {}: {:?}", pathname, e);
            NaiveDateTime::UNIX_EPOCH
        }
    };

    Some(create_file_attribute(size, modify_time, is_dir))
}

fn is_directory(ftp: &mut FtpStream, pathname: &str, detection_method: &DirectoryDetectionMethod) -> bool {
    match detection_method {
        DirectoryDetectionMethod::CwdCdup => {
            if ftp.cwd(pathname).is_ok() {
                let _ = ftp.cdup();
                true
            } else {
                false
            }
        },
        DirectoryDetectionMethod::List => {
            ftp.list(Some(pathname)).is_ok()
        },
        DirectoryDetectionMethod::Mlsd => {
            ftp.mlsd(Some(pathname)).is_ok()
        },
        DirectoryDetectionMethod::FileSize => {
            if ftp.size(pathname).is_err() {
                if let Some(parent) = Path::new(pathname).parent() {
                    if let Ok(list) = ftp.list(Some(parent.to_str().unwrap_or_default())) {
                        for item in list {
                            if item.starts_with('d') && item.ends_with(Path::new(pathname).file_name().unwrap_or_default().to_str().unwrap_or_default()) {
                                return true;
                            }
                        }
                    }
                }
                false
            } else {
                false
            }
        },
    }
}
