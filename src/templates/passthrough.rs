use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use types::FuseDirEntry;

use super::fd_bridge::FileDescriptorBridge;
use crate::types::*;
use crate::*;

pub struct PassthroughFs {
    repo: PathBuf,
    inner: Box<dyn FuseAPI<PathBuf>>
}

impl PassthroughFs {
    pub fn new<T: FuseAPI<PathBuf>>(repo: PathBuf, inner: T) -> Self {
        Self {
            repo,
            inner: Box::new(FileDescriptorBridge::new(inner))
        }
    }
}


impl FuseAPI<PathBuf> for PassthroughFs {
    fn get_inner(&self) -> &Box<(dyn FuseAPI<PathBuf>)> {
        &self.inner
    }

    fn lookup(&self, _req: RequestInfo, parent: PathBuf, name: &OsStr)
        -> FuseResult<FileAttribute> {
        let file_path = self.repo.join(parent).join(name);
        let fd = posix_fs::open(file_path.as_ref(), OpenFlags::empty())?;
        let result = posix_fs::getattr(&fd);
        posix_fs::release(fd)?;
        result
    }

    fn open(
            &self,
            _req: RequestInfo,
            file: PathBuf,
            flags: OpenFlags,
        ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        let file_path = self.repo.join(file);
        let fd = posix_fs::open(file_path.as_ref(), flags)?;
        Ok((fd.to_file_handle()?, FUSEOpenResponseFlags::empty()))
    }

    fn getattr(
            &self,
            _req: RequestInfo,
            file: PathBuf,
            _file_handle: Option<FileHandle>,
        ) -> FuseResult<FileAttribute> {
        let file_path = self.repo.join(file);
        let fd = posix_fs::open(file_path.as_ref(), OpenFlags::empty())?;
        let result = posix_fs::getattr(&fd);
        posix_fs::release(fd)?;
        result
    }

    fn readdir(
            &self,
            _req: RequestInfo,
            file: PathBuf,
            _file_handle: FileHandle,
        ) -> FuseResult<Vec<FuseDirEntry>> {
        let folder_path = self.repo.join(file);
        let children = posix_fs::readdir(folder_path.as_ref())?;
        let mut result = Vec::new();
        result.push(FuseDirEntry {
            inode: INVALID_INODE,
            name: OsString::from("."),
            kind: FileType::Directory,
        });
        result.push(FuseDirEntry {
            inode: INVALID_INODE,
            name: OsString::from(".."),
            kind: FileType::Directory,
        });
        for (child_name, child_kind) in children {
            result.push({
                FuseDirEntry {
                    inode: INVALID_INODE,
                    name: child_name,
                    kind: child_kind,
                }
            })
        }
        Ok(result)
    }

    
    fn listxattr(&self, _req: RequestInfo, file: PathBuf, size: u32) -> FuseResult<Vec<u8>> {
        let file_path = self.repo.join(file);
        posix_fs::listxattr(&file_path, size)
    }

    fn access(&self, _req: RequestInfo, file: PathBuf, mask: AccessMask) -> FuseResult<()> {
        let file_path = self.repo.join(file);
        posix_fs::access(&file_path, mask)
    }

}
