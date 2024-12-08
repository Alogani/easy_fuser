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
    sublayer: FileDescriptorBridge
}

struct BackEndFs {}

impl FilesystemBackend for BackEndFs {
    fn readdir(&self, parent_path: &Path) -> Result<Vec<PathBuf>, io::Error> {
        Ok(posix_fs::readdir(parent_path)?
            .into_iter()
            .map(|(name, _type)| name)
            .collect())
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
        Self {
            path_handler: Mutex::new(InodePathHandler::new(
                Box::new(BackEndFs {}),
                PathBuf::from(repo),
            )),
            sublayer: FileDescriptorBridge::new()
        }
    }
}

impl FuseAPI for PassthroughFs {
    fn get_sublayer(&self) -> &impl FuseAPI {
        &self.sublayer
    }

    fn lookup(
        &self,
        _req: RequestInfo,
        parent_ino: u64,
        name: &OsStr,
        callback: ReplyCb<AttributeResponse>,
    ) {
        callback((|| {
            let mut path_handler = self.path_handler.lock().unwrap();
            let inode = path_handler.lookup(parent_ino, name)?;
            let path = path_handler.get_path(inode)?;
            let fd = posix_fs::open(path.as_ref(), OpenFlags::new())?;
            let result = posix_fs::getattr(&fd, Some(inode));
            posix_fs::release(fd)?;
            result
        })());
    }

    fn open(
            &self,
            _req: RequestInfo,
            ino: u64,
            flags: OpenFlags,
            callback: ReplyCb<(FileHandle, FUSEOpenResponseFlags)>,
        ) {
            callback((|| {
                let path_handler = self.path_handler.lock().unwrap();
                let path = path_handler.get_path(ino)?;
                let fd = posix_fs::open(path.as_ref(), flags)?;
                Ok((fd.to_file_handle()?, FUSEOpenResponseFlags::new()))
            })());
    }

    fn getattr(
        &self,
        _req: RequestInfo,
        ino: u64,
        _file_handle: Option<FileHandle>,
        callback: ReplyCb<AttributeResponse>,
    ) {
        callback((|| {
            let path_handler = self.path_handler.lock().unwrap();
            let path = path_handler.get_path(ino)?;
            let fd = posix_fs::open(path.as_ref(), OpenFlags::new())?;
            let result = posix_fs::getattr(&fd, Some(ino));
            posix_fs::release(fd)?;
            result
        })());
    }

    fn readdir(
        &self,
        _req: RequestInfo,
        ino: u64,
        _file_handle: FileHandle,
        callback: ReplyCb<Vec<FuseDirEntry>>,
    ) {
        callback((|| {
            let mut path_handler = self.path_handler.lock().unwrap();
            let children = path_handler.readdir(ino)?;
            let mut result = Vec::new();
            result.push(FuseDirEntry {
                inode: ino,
                name: OsString::from("."),
                kind: FileType::Directory,
            });
            result.push(FuseDirEntry {
                inode: path_handler.lookup_parent(ino),
                name: OsString::from(".."),
                kind: FileType::Directory,
            });
            for (child_name, child_ino) in children {
                result.push({
                    let file_attr =
                        posix_fs::lookup(path_handler.get_path(child_ino)?.as_ref(), None)?
                            .file_attr;
                    FuseDirEntry {
                        inode: child_ino,
                        name: child_name,
                        kind: file_attr.kind,
                    }
                })
            }
            Ok(result)
        })());
    }

    fn listxattr(&self, _req: RequestInfo, ino: u64, size: u32, callback: ReplyCb<Vec<u8>>) {
        callback((|| {
            let path_handler = self.path_handler.lock().unwrap();
            let path = path_handler.get_path(ino)?;
            posix_fs::listxattr(&path, size)
        })())
    }

    fn access(&self, _req: RequestInfo, ino: u64, mask: AccessMask, callback: ReplyCb<()>) {
        callback((|| {
            let path_handler = self.path_handler.lock().unwrap();
            let path = path_handler.get_path(ino)?;
            posix_fs::access(&path, mask)
        })())
    }

}
