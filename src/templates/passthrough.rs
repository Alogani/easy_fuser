use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use fuse_api::ReplyCb;
use inode_path_handler::{FilesystemBackend, InodePathHandler};
use types::FuseDirEntry;

use super::fd_bridge::FileDescriptorBridge;
use crate::types::*;
use crate::*;

pub struct PassthroughFs {
    path_handler: Mutex<InodePathHandler>,
}

struct BackEndFs {
}

impl FilesystemBackend for BackEndFs {
    fn readdir(&self, parent_path: &Path) -> Result<Vec<PathBuf>, io::Error> {
        Ok(posix_fs::readdir(parent_path)?
            .into_iter()
            .map(|(name, _type)| name)
            .collect()
        )
    }

    fn rename(&self, path: &Path, newpath: &Path, flags: RenameFlags) -> Result<(), io::Error> {
        posix_fs::rename(path, newpath, flags)
    }

    fn unlink(&self, path: &Path) -> Result<(), io::Error> {
        posix_fs::unlink(path)
    }
}

impl PassthroughFs {
    pub fn new(repo: PathBuf) -> Self {
        Self { path_handler: Mutex::new(
            InodePathHandler::new(Box::new(BackEndFs { }), PathBuf::from(repo))
        )}
    }
}

impl FileDescriptorBridge for PassthroughFs {}

impl FuseAPI for PassthroughFs {
    fn lookup(
        &self,
        _req: RequestInfo,
        parent_ino: u64,
        name: &OsStr,
        callback: ReplyCb<AttributeResponse>,
    ) {
        callback((|| {
            let mut path_handler = self.path_handler.lock().unwrap();
            let inode: u64 = path_handler.lookup(parent_ino, name)?;
            let path = path_handler.get_path(inode)?;
            let fd = posix_fs::open(path.as_ref(), OpenFlags::new())?;
            let result = posix_fs::getattr(&fd, Some(inode));
            posix_fs::release(fd)?;
            result
        })());
    }

    fn readdir(
        &self,
        _req: RequestInfo,
        ino: u64,
        file_handle: FileHandle,
        callback: ReplyCb<Vec<FuseDirEntry>>,
    ) {
        callback((|| {
            let mut path_handler = self.path_handler.lock().unwrap();
            let children = path_handler.readdir(ino)?;
            let mut result = Vec::new();
            result.push(FuseDirEntry{inode: ino, name: OsString::from("."), kind: FileType::Directory});
            result.push(FuseDirEntry{inode: path_handler.lookup_parent(ino), name: OsString::from(".."), kind: FileType::Directory});
            for (child_name, child_ino) in children {
                result.push({
                    let file_attr = posix_fs::lookup(path_handler.get_path(child_ino)?.as_ref(), None)?.file_attr;
                    FuseDirEntry{
                        inode: child_ino,
                        name: child_name,
                        kind: file_attr.kind,
                    }
                })
            }
            Ok(result)
        })());
    }
}
