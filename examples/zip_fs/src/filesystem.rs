use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use easy_fuser::templates::DefaultFuseHandler;
use zip::read::{ZipFile, ZipFileSeek};
use zip::ZipArchive;
use std::fs::File;
use std::io::{Read, Seek};
use easy_fuser::prelude::*;

use crate::helpers::{create_file_attribute, get_root_attribute};

pub struct ZipFs {
    archive: RwLock<ZipArchive<File>>,
    inner_fs: DefaultFuseHandler,
}

impl ZipFs {
    pub fn new(zip_path: &Path) -> std::io::Result<Self> {
        let file = File::open(zip_path)?;
        let archive = ZipArchive::new(file)?;
        Ok(Self {
            archive: RwLock::new(archive),
            inner_fs: DefaultFuseHandler::new(),
        })
    }

    fn with_file<F, R>(&self, path: &Path, f: F) -> Option<R>
    where
        F: FnOnce(&mut ZipFile) -> R,
    {
        let path_str = path.to_str()?;
        self.archive.write().ok().and_then(|mut archive| {
            archive.by_name(path_str).ok().map(|mut file| f(&mut file))
        })
    }

    fn with_seekable_file<F, R>(&self, path: &Path, f: F) -> Option<R>
    where
        F: FnOnce(&mut ZipFileSeek<File>) -> R,
    {
        let path_str = path.to_str()?;
        self.archive.write().ok().and_then(|mut archive| {
            archive.by_name_seek(path_str).ok().map(|mut file| f(&mut file))
        })
    }
}

impl FuseHandler<PathBuf> for ZipFs {
    fn get_inner(&self) -> &dyn FuseHandler<PathBuf> {
        &self.inner_fs
    }

    fn lookup(&self, _req: &RequestInfo, parent_id: PathBuf, name: &OsStr) -> FuseResult<FileAttribute> {
        let path = parent_id.join(name);
        self.with_file(&path, |file| create_file_attribute(file))
            .ok_or_else(|| PosixError::new(ErrorKind::FileNotFound, "File not found"))
    }

    fn getattr(&self, _req: &RequestInfo, file_id: PathBuf, _file_handle: Option<FileHandle>) -> FuseResult<FileAttribute> {
        if file_id.is_fuse_root() {
            return Ok(get_root_attribute());
        }
        self.with_file( &file_id, |file| create_file_attribute(file))
            .ok_or_else(|| PosixError::new(ErrorKind::FileNotFound, "File not found"))
    }

    fn read(&self, _req: &RequestInfo, file_id: PathBuf, _file_handle: FileHandle, seek: SeekFrom, size: u32, _flags: FUSEOpenFlags, _lock_owner: Option<u64>) -> FuseResult<Vec<u8>> {
        self.with_seekable_file(&file_id, |file| {
            let mut buffer = vec![0; size as usize];
            file.seek(seek)?;
            let bytes_read = file.read(&mut buffer)?;
            buffer.truncate(bytes_read);
            Ok(buffer)
        }).ok_or_else(|| PosixError::new(ErrorKind::FileNotFound, "File not found"))?
    }

    fn readdir(&self, _req: &RequestInfo, file_id: PathBuf, _file_handle: FileHandle) -> FuseResult<Vec<(OsString, FileKind)>> {
        let mut entries = Vec::new();
        entries.push((OsString::from("."), FileKind::Directory));
        entries.push((OsString::from(".."), FileKind::Directory));

        if let Ok(mut archive) = self.archive.write() {
            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index(i) {
                    let file_path = PathBuf::from(file.name());
                    if file_path.parent() == Some(&file_id) {
                        let name = file_path.file_name().unwrap().to_owned();
                        eprintln!("File name = {:?}", name);
                        let kind = if file.is_dir() { FileKind::Directory } else { FileKind::RegularFile };
                        entries.push((name, kind));
                    }
                }
            }
        }

        Ok(entries)
    }

}

