use super::super::file_descriptor_handler::{
    file_descriptor_handler_readonly_methods, file_descriptor_handler_readwrite_methods,
};
use super::*;

fn into_file_handle(fd: std::os::fd::OwnedFd) -> FuseResult<OwnedFileHandle> {
    OwnedFileHandle::from_owned_fd(fd)
        .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("could not convert file descriptor"))
}

impl OverlayFs {
    pub fn access(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        mask: AccessFlags,
    ) -> FuseResult<()> {
        let entry = self
            .resolve(&file_id)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("overlay entry does not exist"))?;
        unix_fs::access(&entry.path, mask)
    }

    pub fn getattr(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        file_handle: Option<BorrowedFileHandle<'_>>,
    ) -> FuseResult<FileAttribute> {
        if let Some(file_handle) = file_handle {
            return unix_fs::getattr(file_handle.as_borrowed_fd());
        }
        let entry = self
            .resolve(&file_id)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("overlay entry does not exist"))?;
        unix_fs::lookup(&entry.path)
    }

    pub fn getxattr(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        name: &OsStr,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        let entry = self
            .resolve(&file_id)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("overlay entry does not exist"))?;
        unix_fs::getxattr(&entry.path, name, size)
    }

    pub fn listxattr(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        let entry = self
            .resolve(&file_id)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("overlay entry does not exist"))?;
        unix_fs::listxattr(&entry.path, size)
    }

    pub fn link(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        newparent: PathBuf,
        newname: &OsStr,
    ) -> FuseResult<FileAttribute> {
        let _guard = self.mutation_guard()?;
        let destination_path = self.child_path(&newparent, newname)?;
        self.link_path(&file_id, &destination_path)
    }

    pub fn lookup(
        &self,
        _req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
    ) -> FuseResult<FileAttribute> {
        let path = self.child_path(&parent_id, name)?;
        let entry = self
            .resolve(&path)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("overlay entry does not exist"))?;
        unix_fs::lookup(&entry.path)
    }

    pub fn open(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        flags: OpenFlags,
    ) -> FuseResult<(OwnedFileHandle, FopenFlags)> {
        let path = self.normalize_path(&file_id)?;
        if flags.0 & libc::O_ACCMODE != libc::O_RDONLY {
            let _guard = self.mutation_guard()?;
            self.copy_up(&path)?;
        }
        let entry = self
            .resolve(&path)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("overlay entry does not exist"))?;
        let fd = unix_fs::open(&entry.path, flags)?;
        let file_handle = into_file_handle(fd)?;
        Ok((file_handle, FopenFlags::empty()))
    }

    pub fn readdir(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        _file_handle: BorrowedFileHandle<'_>,
    ) -> FuseResult<Vec<(OsString, FileKind)>> {
        self.readdir_entries(&file_id)
    }

    pub fn readdirplus(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        file_handle: BorrowedFileHandle<'_>,
    ) -> FuseResult<Vec<(OsString, FileAttribute)>> {
        let entries = self.readdir(req, file_id.clone(), file_handle)?;
        entries
            .into_iter()
            .map(|(name, _)| {
                let attribute = match name.as_os_str() {
                    name if name == OsStr::new(".") => {
                        self.getattr(req, file_id.clone(), None)?
                    }
                    name if name == OsStr::new("..") => {
                        let parent = file_id.parent().unwrap_or(Path::new(""));
                        self.getattr(req, parent.to_path_buf(), None)?
                    }
                    _ => self.lookup(req, file_id.clone(), &name)?,
                };
                Ok((name, attribute))
            })
            .collect()
    }

    pub fn bmap(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        blocksize: u32,
        index: u64,
    ) -> FuseResult<u64> {
        let entry = self
            .resolve(&file_id)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("overlay entry does not exist"))?;
        unix_fs::bmap(&entry.path, blocksize, index)
    }

    pub fn readlink(&self, _req: &RequestInfo, file_id: PathBuf) -> FuseResult<Vec<u8>> {
        let entry = self
            .resolve(&file_id)?
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("overlay entry does not exist"))?;
        unix_fs::readlink(&entry.path)
    }

    pub fn statfs(&self, _req: &RequestInfo, _file_id: PathBuf) -> FuseResult<StatFs> {
        unix_fs::statfs(&self.upper_dir)
    }

    pub fn create(
        &self,
        _req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
    ) -> FuseResult<(OwnedFileHandle, FileAttribute, FopenFlags)> {
        let _guard = self.mutation_guard()?;
        let path = self.child_path(&parent_id, name)?;
        if let Some(entry) = self.resolve(&path)? {
            if flags.0 & libc::O_EXCL != 0 {
                return Err(ErrorKind::FileExists.to_error("overlay entry already exists"));
            }
            if !entry.from_upper {
                self.copy_up(&path)?;
            }
        }
        self.ensure_upper_parents(&path)?;
        let upper_path = self.upper_dir.join(&path);
        let (fd, attributes) = unix_fs::create(&upper_path, mode, umask, flags)?;
        let file_handle = into_file_handle(fd)?;
        Ok((file_handle, attributes, FopenFlags::empty()))
    }

    pub fn mkdir(
        &self,
        _req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
        mode: u32,
        umask: u32,
    ) -> FuseResult<FileAttribute> {
        let _guard = self.mutation_guard()?;
        let path = self.child_path(&parent_id, name)?;
        if self.resolve(&path)?.is_some() {
            return Err(ErrorKind::FileExists.to_error("overlay entry already exists"));
        }
        self.ensure_upper_parents(&path)?;
        unix_fs::mkdir(&self.upper_dir.join(path), mode, umask)
    }

    pub fn mknod(
        &self,
        _req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: DeviceType,
    ) -> FuseResult<FileAttribute> {
        let _guard = self.mutation_guard()?;
        let path = self.child_path(&parent_id, name)?;
        if self.resolve(&path)?.is_some() {
            return Err(ErrorKind::FileExists.to_error("overlay entry already exists"));
        }
        self.ensure_upper_parents(&path)?;
        unix_fs::mknod(&self.upper_dir.join(path), mode, umask, rdev)
    }

    pub fn removexattr(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        name: &OsStr,
    ) -> FuseResult<()> {
        let _guard = self.mutation_guard()?;
        let path = self.normalize_path(&file_id)?;
        let upper_path = self.copy_up_for_metadata(&path)?;
        unix_fs::removexattr(&upper_path, name)
    }

    pub fn rename(
        &self,
        _req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
        newparent: PathBuf,
        newname: &OsStr,
        flags: RenameFlags,
    ) -> FuseResult<()> {
        let _guard = self.mutation_guard()?;
        let old_path = self.child_path(&parent_id, name)?;
        let new_path = self.child_path(&newparent, newname)?;
        self.rename_path(&old_path, &new_path, flags)
    }

    pub fn rmdir(&self, _req: &RequestInfo, parent_id: PathBuf, name: &OsStr) -> FuseResult<()> {
        let _guard = self.mutation_guard()?;
        let path = self.child_path(&parent_id, name)?;
        self.rmdir_path(&path)
    }

    pub fn setattr(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        attrs: SetAttrRequest<'_>,
    ) -> FuseResult<FileAttribute> {
        let _guard = self.mutation_guard()?;
        let path = self.normalize_path(&file_id)?;
        let upper_path = self.copy_up_for_metadata(&path)?;
        unix_fs::setattr(&upper_path, attrs)
    }

    pub fn setxattr(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        name: &OsStr,
        value: Vec<u8>,
        flags: SetXAttrFlags,
        position: u32,
    ) -> FuseResult<()> {
        let _guard = self.mutation_guard()?;
        let path = self.normalize_path(&file_id)?;
        let upper_path = self.copy_up_for_metadata(&path)?;
        unix_fs::setxattr(&upper_path, name, &value, flags, position)
    }

    pub fn symlink(
        &self,
        _req: &RequestInfo,
        parent_id: PathBuf,
        link_name: &OsStr,
        target: &Path,
    ) -> FuseResult<FileAttribute> {
        let _guard = self.mutation_guard()?;
        let path = self.child_path(&parent_id, link_name)?;
        if self.resolve(&path)?.is_some() {
            return Err(ErrorKind::FileExists.to_error("overlay entry already exists"));
        }
        self.ensure_upper_parents(&path)?;
        unix_fs::symlink(&self.upper_dir.join(path), target)
    }

    pub fn unlink(&self, _req: &RequestInfo, parent_id: PathBuf, name: &OsStr) -> FuseResult<()> {
        let _guard = self.mutation_guard()?;
        let path = self.child_path(&parent_id, name)?;
        self.unlink_path(&path)
    }

    file_descriptor_handler_readonly_methods!(PathBuf);
    file_descriptor_handler_readwrite_methods!(PathBuf);
}
