use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use crate::prelude::*;
use crate::templates::FdHandlerHelper;

/// Mirror the content of another folder in a read-write manner
///
/// The following functions are not implemented:
/// - link
/// - setlk
/// - getlk
/// - bmap
/// - ioctl
pub struct MirrorFs {
    repo: PathBuf,
    inner: Box<dyn FuseHandler<PathBuf>>,
}

impl MirrorFs {
    pub fn new<T: FuseHandler<PathBuf>>(repo: PathBuf, inner: T) -> Self {
        Self {
            repo,
            inner: Box::new(FdHandlerHelper::new(inner)),
        }
    }
}

impl FuseHandler<PathBuf> for MirrorFs {
    fn get_inner(&self) -> &Box<(dyn FuseHandler<PathBuf>)> {
        &self.inner
    }

    fn lookup(
        &self,
        _req: RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
    ) -> FuseResult<FileAttribute> {
        let file_path = self.repo.join(parent_id).join(name);
        let fd = posix_fs::open(file_path.as_ref(), OpenFlags::empty())?;
        let result = posix_fs::getattr(&fd);
        result
    }

    fn getattr(
        &self,
        _req: RequestInfo,
        file_id: PathBuf,
        _file_handle: Option<FileHandle>,
    ) -> FuseResult<FileAttribute> {
        let file_path = self.repo.join(file_id);
        let fd = posix_fs::open(file_path.as_ref(), OpenFlags::empty())?;
        let result = posix_fs::getattr(&fd);
        result
    }

    fn setattr(
        &self,
        _req: RequestInfo,
        file_id: PathBuf,
        attrs: SetAttrRequest,
    ) -> FuseResult<FileAttribute> {
        let file_path = self.repo.join(file_id);
        posix_fs::setattr(&file_path, attrs)
    }

    fn readlink(&self, _req: RequestInfo, file_id: PathBuf) -> FuseResult<Vec<u8>> {
        let file_path = self.repo.join(file_id);
        posix_fs::readlink(&file_path)
    }

    fn mknod(
            &self,
            _req: RequestInfo,
            parent_id: PathBuf,
            name: &OsStr,
            mode: u32,
            umask: u32,
            rdev: DeviceType,
        ) -> FuseResult<FileAttribute> {
        let file_path = self.repo.join(parent_id).join(name);
        posix_fs::mknod(&file_path, mode, umask, rdev)
    }

    fn mkdir(
            &self,
            _req: RequestInfo,
            parent_id: PathBuf,
            name: &OsStr,
            mode: u32,
            umask: u32,
        ) -> FuseResult<FileAttribute> {
        let file_path = self.repo.join(parent_id).join(name);
        posix_fs::mkdir(&file_path, mode, umask)
    }

    fn unlink(&self, _req: RequestInfo, parent_id: PathBuf, name: &OsStr) -> FuseResult<()> {
        let file_path = self.repo.join(parent_id).join(name);
        posix_fs::unlink(&file_path)
    }

    fn symlink(
            &self,
            _req: RequestInfo,
            parent_id: PathBuf,
            link_name: &OsStr,
            target: &std::path::Path,
        ) -> FuseResult<FileAttribute> {
        let file_path = self.repo.join(parent_id).join(link_name);
        posix_fs::symlink(&file_path, target)
    }

    fn rename(
            &self,
            _req: RequestInfo,
            parent_id: PathBuf,
            name: &OsStr,
            newparent: PathBuf,
            newname: &OsStr,
            flags: RenameFlags,
        ) -> FuseResult<()> {
        let oldpath = self.repo.join(parent_id).join(name);
        let newpath = self.repo.join(newparent).join(newname);
        posix_fs::rename(&oldpath, &newpath, flags)
    }

    fn open(
        &self,
        _req: RequestInfo,
        file_id: PathBuf,
        flags: OpenFlags,
    ) -> FuseResult<(FileHandle, FUSEOpenResponseFlags)> {
        let file_path = self.repo.join(file_id);
        let mut fd = posix_fs::open(file_path.as_ref(), flags)?;
        Ok((fd.take_to_file_handle()?, FUSEOpenResponseFlags::empty()))
    }

    fn readdir(
        &self,
        _req: RequestInfo,
        file_id: PathBuf,
        _file_handle: FileHandle,
    ) -> FuseResult<Vec<FuseDirEntry>> {
        let folder_path = self.repo.join(file_id);
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

    fn statfs(&self, _req: RequestInfo, file_id: PathBuf) -> FuseResult<StatFs> {
        let file_path = self.repo.join(file_id);
        posix_fs::statfs(&file_path)
    }

    fn setxattr(
        &self,
        _req: RequestInfo,
        file_id: PathBuf,
        name: &OsStr,
        value: &[u8],
        _flags: FUSESetXAttrFlags,
        position: u32,
    ) -> FuseResult<()> {
        let file_path = self.repo.join(file_id);
        posix_fs::setxattr(&file_path, name, value, position)
    }

    fn getxattr(
        &self,
        _req: RequestInfo,
        file_id: PathBuf,
        name: &OsStr,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        let file_path = self.repo.join(file_id);
        posix_fs::getxattr(&file_path, name, size)
    }

    fn listxattr(&self, _req: RequestInfo, file_id: PathBuf, size: u32) -> FuseResult<Vec<u8>> {
        let file_path = self.repo.join(file_id);
        posix_fs::listxattr(&file_path, size)
    }

    fn access(&self, _req: RequestInfo, file_id: PathBuf, mask: AccessMask) -> FuseResult<()> {
        let file_path = self.repo.join(file_id);
        posix_fs::access(&file_path, mask)
    }

    fn create(
            &self,
            _req: RequestInfo,
            parent_id: PathBuf,
            name: &OsStr,
            mode: u32,
            umask: u32,
            flags: OpenFlags,
        ) -> FuseResult<(FileHandle, FileAttribute, FUSEOpenResponseFlags)> {
        let file_path = self.repo.join(parent_id).join(name);
        let (mut fd, file_attr) = posix_fs::create(&file_path, mode, umask, flags)?;
        Ok((fd.take_to_file_handle()?, file_attr, FUSEOpenResponseFlags::empty()))
    }
}
