use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use types::FuseDirEntry;

use super::fd_bridge::FileDescriptorBridge;
use crate::types::*;
use crate::*;



impl PassthroughFs {
    pub fn new(repo: PathBuf) -> Self {
        Self {
            sublayer: FileDescriptorBridge::new()
        }
    }
}

pub struct PassthroughFs {
    sublayer: FileDescriptorBridge
}

impl FuseAPI<PathBuf> for PassthroughFs {
    fn get_inner(&self) -> &impl FuseAPI<PathBuf> {
        &self.sublayer
    }

    fn lookup(&self, req: RequestInfo, parent: PathBuf, name: &OsStr)
        -> FuseResult<FileAttribute> {
        let fd = posix_fs::open(parent.as_ref(), OpenFlags::new())?;
        let result = posix_fs::getattr(&fd);
        posix_fs::release(fd)?;
        result
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
