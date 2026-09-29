use zip::ZipArchive;

use easy_fuser::fuse_presets::{StatelessHandler, UnimplementedFuseHandler};
use easy_fuser::fuse_serial::prelude::*;
use easy_fuser::inode_mapping::*;

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{Cursor, Read, Seek};
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
    unimplemented: UnimplementedFuseHandler<Inode>,
    safe_defaults: StatelessHandler<Inode>,
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
            unimplemented: UnimplementedFuseHandler::new(),
            safe_defaults: StatelessHandler::new(),
        })
    }
}

impl FuseHandler for ZipFs {
    type TId = Inode;
    type FileHandle = Cursor<Vec<u8>>;

    easy_fuser::delegate_fs! { safe_defaults, [ fsyncdir, opendir, releasedir ] }
    easy_fuser::delegate_fs! { unimplemented, [ access, bmap, copy_file_range, create, fallocate, flush, fsync, getlk, getxattr, ioctl, link, listxattr, lseek, mkdir, mknod, readlink, removexattr, rename, rmdir, setattr, setlk, setxattr, statfs, symlink, write, unlink ] }

    fn getattr(
        &self,
        _req: &RequestInfo,
        file_id: Inode,
        _file_handle: Option<&mut Self::FileHandle>,
    ) -> FuseResult<FileAttribute> {
        if file_id.is_filesystem_root() {
            return Ok(get_root_attribute());
        }
        let InodeInfo {
            data: &(idx, is_dir),
            ..
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

    fn open(
        &self,
        _req: &RequestInfo,
        file_id: Inode,
        _flags: OpenFlags,
    ) -> FuseResult<(Self::FileHandle, FopenFlags)> {
        let index = {
            let mapper = self.mapper.read().unwrap();
            let InodeInfo {
                data: &(index, is_dir),
                ..
            } = mapper
                .get(&file_id)
                .ok_or_else(|| ErrorKind::FileNotFound.to_error("inode not found"))?;
            if is_dir {
                return Err(
                    ErrorKind::InvalidArgument.to_error("cannot open a directory as a file")
                );
            }
            index
        };
        let mut archive = self.archive.lock().unwrap();
        let mut entry = archive.by_index(index)?;
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        Ok((Cursor::new(data), FopenFlags::empty()))
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
        _file_id: Inode,
        file_handle: Option<&mut Self::FileHandle>,
        seek: SeekFrom,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        let file = file_handle
            .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing file handle"))?;
        let mut buffer = vec![0; size as usize];
        file.seek(seek)?;
        let bytes_read = file.read(&mut buffer)?;
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
            .map(|(name, inode)| {
                let InodeInfo {
                    data: &(_, is_dir), ..
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

#[cfg(test)]
mod typed_open_resource_tests {
    use super::*;
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    #[test]
    fn reads_from_the_resource_created_by_open() {
        let archive_file = tempfile::NamedTempFile::new().unwrap();
        let mut writer = ZipWriter::new(File::create(archive_file.path()).unwrap());
        writer
            .start_file("first.txt", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"wrong entry").unwrap();
        writer
            .start_file("second.txt", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"opened entry").unwrap();
        writer.finish().unwrap();

        let fs = ZipFs::new(archive_file.path()).unwrap();
        let (first_id, second_id) = {
            let mapper = fs.mapper.read().unwrap();
            let root = mapper.get_root_inode();
            (
                mapper
                    .lookup(&root, OsStr::new("first.txt"))
                    .unwrap()
                    .inode
                    .clone(),
                mapper
                    .lookup(&root, OsStr::new("second.txt"))
                    .unwrap()
                    .inode
                    .clone(),
            )
        };
        let request = RequestInfo {
            id: RequestId(0),
            uid: 0,
            gid: 0,
            pid: 0,
        };
        let (mut file, _) = fs.open(&request, second_id, OpenFlags(0)).unwrap();

        let data = fs
            .read(
                &request,
                first_id,
                Some(&mut file),
                SeekFrom::Start(0),
                32,
                OpenFlags(0),
                None,
            )
            .unwrap();
        assert_eq!(data, b"opened entry");
    }
}
