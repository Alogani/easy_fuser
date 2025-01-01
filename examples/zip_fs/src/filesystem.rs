use zip::ZipArchive;

use easy_fuser::prelude::*;
use easy_fuser::templates::DefaultFuseHandler;

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::helpers::*;

/// Limitations of Zip:
/// - do not support lookup using a parent
/// - dir contains a trailing slash
pub struct ZipFs {
    archive: Mutex<ZipArchive<File>>,
    // HashMap<index, is_dir>
    index_cache: Mutex<HashMap<PathBuf, bool>>,
    inner_fs: DefaultFuseHandler,
}

impl ZipFs {
    pub fn new(zip_path: &Path) -> std::io::Result<Self> {
        let file = File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;
        // To circumvet the limits of Zip, we will index all files in the archive
        let mut index_cache = HashMap::new();
        for i in 0..archive.len() {
            let f = archive.by_index(i).unwrap();
            index_cache.insert(i, f.is_dir());
        }

        Ok(Self {
            archive: Mutex::new(archive),
            index_cache: Mutex::new(index_cache),
            inner_fs: DefaultFuseHandler::new(),
        })
    }

    fn to_index(inode: Inode) -> usize {
        u64::from(inode) - 2
    }

    fn to_inode(index: u64) -> usize {
        Inode::from(index + 2)
    }

    fn with_file<T, F, R>(&self, path: &Path, func: F) -> Option<R>
    where
        T: ZipExtractor,
        F: for<'a> FnOnce(&mut T::Output<'a>) -> R,
    {
        let path_str = path.to_str()?;

        let mut archive = self.archive.lock().ok()?;
        // Try to find the file without the trailing slash
        if let Some(ref mut file) = T::get_by_name(&mut archive, path_str) {
            return Some(func(file));
        }

        // If not found, try with a trailing slash (for directories)
        let path_str_with_slash = format!("{}/", path_str);
        let result = if let Some(ref mut file) = T::get_by_name(&mut archive, &path_str_with_slash)
        {
            // If found with trailing slash, update the folder cache
            self.folder_cache.insert(path.to_path_buf(), ());
            Some(func(file))
        } else {
            None
        };
        return result;
    }
}

impl FuseHandler<PathBuf> for ZipFs {
    fn get_inner(&self) -> &dyn FuseHandler<PathBuf> {
        &self.inner_fs
    }

    fn getattr(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        _file_handle: Option<FileHandle>,
    ) -> FuseResult<FileAttribute> {
        if file_id.is_filesystem_root() {
            return Ok(get_root_attribute());
        }
        let mut archive = self.archive.lock()?;
        /*self.with_file::<NonSeekable, _, _>(&file_id, |file| create_file_attribute(file))
            .ok_or_else(|| PosixError::new(ErrorKind::FileNotFound, "File not found"))
            */
    }

    fn lookup(
        &self,
        _req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
    ) -> FuseResult<FileAttribute> {
        let path = parent_id.join(name);
        self.with_file::<NonSeekable, _, _>(&path, |file| create_file_attribute(file))
            .ok_or_else(|| PosixError::new(ErrorKind::FileNotFound, "File not found"))
    }

    fn read(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        _file_handle: FileHandle,
        seek: SeekFrom,
        size: u32,
        _flags: FUSEOpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        self.with_file::<Seekable, _, _>(&file_id, |file| {
            let mut buffer = vec![0; size as usize];
            file.seek(seek)?;
            let bytes_read = file.read(&mut buffer)?;
            buffer.truncate(bytes_read);
            Ok(buffer)
        })
        .ok_or_else(|| PosixError::new(ErrorKind::FileNotFound, "File not found"))?
    }

    fn readdir(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        _file_handle: FileHandle,
    ) -> FuseResult<Vec<(OsString, FileKind)>> {
        let mut entries = Vec::new();
        entries.push((OsString::from("."), FileKind::Directory));
        entries.push((OsString::from(".."), FileKind::Directory));

        if let Ok(mut archive) = self.archive.lock() {
            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index(i) {
                    let file_path = file.enclosed_name().unwrap();
                    if file_path.parent() == Some(&file_id) {
                        let mut name_bytes = file_path.into_os_string().into_encoded_bytes();
                        let name = unsafe {
                            if name_bytes[name_bytes.len() - 1] == b'/' {
                                name_bytes.pop();
                                let name = OsString::from_encoded_bytes_unchecked(name_bytes);
                                self.folder_cache.insert(file_id.join(name.clone()), ());
                                name
                            } else {
                                OsString::from_encoded_bytes_unchecked(name_bytes)
                            }
                        };
                        let kind = if file.is_dir() {
                            FileKind::Directory
                        } else {
                            FileKind::RegularFile
                        };
                        entries.push((name, kind));
                    }
                }
            }
        }

        Ok(entries)
    }
}
