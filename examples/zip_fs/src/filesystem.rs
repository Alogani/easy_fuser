use zip::ZipArchive;

use easy_fuser::inode_mapper::*;
use easy_fuser::prelude::*;
use easy_fuser::templates::DefaultFuseHandler;

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;
use std::sync::{Mutex, RwLock};

use crate::helpers::*;

/// Limitations of Zip:
/// - do not support lookup using a parent
/// - dir contains a trailing slash
pub struct ZipFs {
    archive: Mutex<ZipArchive<File>>,
    // index and is_dir are stored in a tuple
    mapper: RwLock<InodeMapper<(usize, bool)>>,
    inner_fs: DefaultFuseHandler,
}

impl ZipFs {
    pub fn new(zip_path: &Path) -> std::io::Result<Self> {
        let file = File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;
        // To circumvet the limits of Zip, we will index all files in the archive
        let archive_len = archive.len();
        let mut entries = Vec::with_capacity(archive_len);
        for idx in 0..archive_len {
            let file_path = archive.by_index(idx).unwrap().name_raw().to_vec();
            let (components, data) = {
                let mut path_iter = file_path.into_iter();
                let mut components = Vec::new();
                let mut acc = Vec::new();
                let mut is_dir = true;
                loop {
                    let c = path_iter.next();
                    match c {
                        None => {
                            if !acc.is_empty() {
                                components
                                    .push(unsafe { OsString::from_encoded_bytes_unchecked(acc) });
                                is_dir = false;
                            }
                            break;
                        }
                        Some(b'/') => {
                            if !acc.is_empty() {
                                components
                                    .push(unsafe { OsString::from_encoded_bytes_unchecked(acc) });
                                acc = Vec::new();
                            }
                        }
                        Some(c) => {
                            acc.push(c);
                        }
                    }
                }
                (components, (idx, is_dir))
            };
            entries.push((components, move |_: ValueCreatorParams<(usize, bool)>| data));
        }

        let mut mapper = InodeMapper::new((0, true));
        mapper
            .batch_insert(&mapper.get_root_inode(), entries, |_| {
                panic!("archive contains orphan childs")
            })
            .expect("Failed to batch insert entries");

        Ok(Self {
            archive: Mutex::new(archive),
            mapper: RwLock::new(mapper),
            inner_fs: DefaultFuseHandler::new(),
        })
    }
}

impl FuseHandler<Inode> for ZipFs {
    fn get_inner(&self) -> &dyn FuseHandler<Inode> {
        &self.inner_fs
    }

    fn getattr(
        &self,
        _req: &RequestInfo,
        file_id: Inode,
        _file_handle: Option<BorrowedFileHandle>,
    ) -> FuseResult<FileAttribute> {
        if file_id.is_filesystem_root() {
            return Ok(get_root_attribute());
        }
        let InodeInfo {
            parent: _,
            name: _,
            data: &(idx, is_dir),
        } = self
            .mapper
            .read()
            .unwrap()
            .get(&file_id)
            .expect("inode not found");
        let mut archive = self.archive.lock().unwrap();
        let file_attr = create_file_attribute(&archive.by_index(idx)?, is_dir);
        Ok(file_attr)
    }

    fn lookup(
        &self,
        _req: &RequestInfo,
        parent_id: Inode,
        name: &OsStr,
    ) -> FuseResult<(Inode, FileAttribute)> {
        let binding = self.mapper.read().unwrap();
        let LookupResult {
            inode,
            name: _,
            data: &(idx, is_dir),
        } = binding
            .lookup(&parent_id, name)
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("File not found"))?;

        let mut archive = self.archive.lock().unwrap();
        let file_attr = create_file_attribute(&archive.by_index(idx)?, is_dir);
        Ok((inode.clone(), file_attr))
    }

    fn read(
        &self,
        _req: &RequestInfo,
        file_id: Inode,
        _file_handle: BorrowedFileHandle,
        seek: SeekFrom,
        size: u32,
        _flags: FUSEOpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        let InodeInfo {
            parent: _,
            name: _,
            data: &(idx, _),
        } = self
            .mapper
            .read()
            .unwrap()
            .get(&file_id)
            .expect("inode not found");
        let mut archive = self.archive.lock().unwrap();
        let mut zip_file = archive.by_index_seek(idx)?;
        let mut buffer = vec![0; size as usize];
        zip_file.seek(seek)?;
        let bytes_read = zip_file.read(&mut buffer)?;
        buffer.truncate(bytes_read);
        Ok(buffer)
    }

    fn readdir(
        &self,
        _req: &RequestInfo,
        file_id: Inode,
        _file_handle: BorrowedFileHandle,
    ) -> FuseResult<Vec<(OsString, (Inode, FileKind))>> {
        let mapper = self.mapper.read().unwrap();
        let entries = mapper
            .get_children(&file_id)
            .into_iter()
            .map(|(_, inode)| {
                let InodeInfo {
                    parent: _,
                    name,
                    data: &(_, is_dir),
                } = mapper.get(inode).unwrap();
                (
                    (**name).clone(),
                    (
                        inode.clone(),
                        if is_dir {
                            FileKind::Directory
                        } else {
                            FileKind::RegularFile
                        },
                    ),
                )
            })
            .collect();
        Ok(entries)
    }
}
