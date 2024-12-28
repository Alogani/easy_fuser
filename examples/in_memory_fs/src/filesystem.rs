use easy_fuser::prelude::*;
use easy_fuser::templates::DefaultFuseHandler;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct InMemoryFS {
    inner: DefaultFuseHandler,
    fs: Arc<Mutex<DataBank>>,
}

struct DataBank {
    inodes: HashMap<Inode, FSNode>,
    next_inode: Inode,
}

struct FSNode {
    parent: Inode,
    attr: FileAttribute,
    data: Vec<u8>,
    children: HashMap<OsString, Inode>,
}

impl InMemoryFS {
    pub fn new() -> Self {
        let mut fs = DataBank {
            inodes: HashMap::new(),
            next_inode: Inode::from(2), // Root is 1
        };

        // Create root directory
        fs.inodes.insert(
            ROOT_INODE,
            FSNode {
                parent: ROOT_INODE,
                attr: FileAttribute {
                    size: 0,
                    blocks: 1,
                    atime: UNIX_EPOCH,
                    mtime: UNIX_EPOCH,
                    ctime: UNIX_EPOCH,
                    crtime: UNIX_EPOCH,
                    kind: FileKind::Directory,
                    perm: 0o755,
                    nlink: 2,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    flags: 0,
                    blksize: 512,
                    ttl: None,
                    generation: None,
                },
                data: Vec::new(),
                children: HashMap::new(),
            },
        );

        Self {
            inner: DefaultFuseHandler::new(),
            fs: Arc::new(Mutex::new(fs)),
        }
    }
}

impl FuseHandler<Inode> for InMemoryFS {
    fn get_inner(&self) -> &dyn FuseHandler<Inode> {
        &self.inner
    }

    fn lookup(&self, _req: &RequestInfo, parent: Inode, name: &OsStr) -> FuseResult<(Inode, FileAttribute)> {
        let fs = self.fs.lock().unwrap();
        if let Some(parent_node) = fs.inodes.get(&parent) {
            if let Some(child_inode) = parent_node.children.get(name) {
                if let Some(child_node) = fs.inodes.get(&child_inode) {
                    return Ok((child_inode.clone(), child_node.attr.clone()));
                }
            }
        }
        Err(ErrorKind::FileNotFound.to_error(""))
    }

    fn getattr(&self, _req: &RequestInfo, ino: Inode, _fh: Option<FileHandle>) -> FuseResult<FileAttribute> {
        let fs = self.fs.lock().unwrap();
        fs.inodes.get(&ino)
            .map(|node| node.attr.clone())
            .ok_or_else(|| ErrorKind::FileNotFound.to_error(""))
    }

    fn setattr(
        &self,
        _req: &RequestInfo,
        file_id: Inode,
        attrs: SetAttrRequest,
    ) -> FuseResult<FileAttribute> {
        let mut fs = self.fs.lock().unwrap();
        
        if let Some(node) = fs.inodes.get_mut(&file_id) {
            // Update mode if provided
            if let Some(new_mode) = attrs.mode {
                node.attr.perm = new_mode as u16;
            }

            // Update uid if provided
            if let Some(new_uid) = attrs.uid {
                node.attr.uid = new_uid;
            }

            // Update gid if provided
            if let Some(new_gid) = attrs.gid {
                node.attr.gid = new_gid;
            }

            // Update size if provided
            if let Some(new_size) = attrs.size {
                node.attr.size = new_size;
                if new_size as usize > node.data.len() {
                    node.data.resize(new_size as usize, 0);
                } else {
                    node.data.truncate(new_size as usize);
                }
            }

            // Update atime if provided
            if let Some(new_atime) = attrs.atime {
                node.attr.atime = match new_atime {
                    TimeOrNow::SpecificTime(time) => time,
                    TimeOrNow::Now => SystemTime::now(),
                };
            }

            // Update mtime if provided
            if let Some(new_mtime) = attrs.mtime {
                node.attr.mtime = match new_mtime {
                    TimeOrNow::SpecificTime(time) => time,
                    TimeOrNow::Now => SystemTime::now(),
                };
            }

            // Always update ctime when attributes are changed
            node.attr.ctime = SystemTime::now();

            Ok(node.attr.clone())
        } else {
            Err(ErrorKind::FileNotFound.to_error(""))
        }
    }


    fn read(&self, _req: &RequestInfo, ino: Inode, _fh: FileHandle, offset: SeekFrom, size: u32, _flags: FUSEOpenFlags, _lock_owner: Option<u64>) -> FuseResult<Vec<u8>> {
        let fs = self.fs.lock().unwrap();
        if let Some(node) = fs.inodes.get(&ino) {
            let offset = match offset {
                SeekFrom::Start(o) => o as usize,
                _ => return Err(ErrorKind::InvalidArgument.to_error("Invalid offset")),
            };
            Ok(node.data[offset..].iter().take(size as usize).cloned().collect())
        } else {
            Err(ErrorKind::FileNotFound.to_error(""))
        }
    }

    fn write(&self, _req: &RequestInfo, ino: Inode, _fh: FileHandle, offset: SeekFrom, data: Vec<u8>, _write_flags: FUSEWriteFlags, _flags: OpenFlags, _lock_owner: Option<u64>) -> FuseResult<u32> {
        let mut fs = self.fs.lock().unwrap();
        if let Some(node) = fs.inodes.get_mut(&ino) {
            let offset = match offset {
                SeekFrom::Start(o) => o as usize,
                _ => return Err(ErrorKind::InvalidArgument.to_error("Invalid offset")),
            };
            if offset + data.len() > node.data.len() {
                node.data.resize(offset + data.len(), 0);
            }
            node.data[offset..offset + data.len()].copy_from_slice(&data);
            node.attr.size = node.data.len() as u64;
            node.attr.mtime = SystemTime::now();
            Ok(data.len() as u32)
        } else {
            Err(ErrorKind::FileNotFound.to_error(""))
        }
    }

    fn mkdir(&self, req: &RequestInfo, parent: Inode, name: &OsStr, mode: u32, _umask: u32) -> FuseResult<(Inode, FileAttribute)> {
        let mut fs = self.fs.lock().unwrap();
        let new_inode = fs.next_inode.clone();
        if let Some(parent_node) = fs.inodes.get_mut(&parent) {
            let attr = FileAttribute {
                size: 0,
                blocks: 1,
                atime: SystemTime::now(),
                mtime: SystemTime::now(),
                ctime: SystemTime::now(),
                crtime: SystemTime::now(),
                kind: FileKind::Directory,
                perm: mode as u16,
                nlink: 1,
                uid: req.uid,
                gid: req.gid,
                rdev: 0,
                flags: 0,
                blksize: 512,
                ttl: None,
                generation: None,
            };

            let new_node = FSNode {
                parent,
                attr: attr.clone(),
                data: Vec::new(),
                children: HashMap::new(),
            };

            parent_node.children.insert(name.to_owned(), new_inode.clone());
            fs.inodes.insert(new_inode.clone(), new_node);
            fs.next_inode = new_inode.next();

            Ok((new_inode.clone(), attr))
        } else {
            Err(ErrorKind::FileNotFound.to_error(""))
        }
    }

    fn readdir(&self, _req: &RequestInfo, ino: Inode, _fh: FileHandle) -> FuseResult<Vec<(OsString, (Inode, FileKind))>> {
        let fs = self.fs.lock().unwrap();
        if let Some(node) = fs.inodes.get(&ino) {
            let mut entries = vec![
                (OsString::from("."), (ino, FileKind::Directory)),
                (OsString::from(".."), (node.parent.clone(), FileKind::Directory)),
            ];
            entries.extend(node.children.iter().map(|(name, child_ino)| {
                let child_node = fs.inodes.get(&child_ino).unwrap();
                (name.clone(), (child_ino.clone(), child_node.attr.kind))
            }));
            Ok(entries)
        } else {
            Err(ErrorKind::FileNotFound.to_error(""))
        }
    }

    fn create(&self, req: &RequestInfo, parent: Inode, name: &OsStr, mode: u32, _umask: u32, _flags: OpenFlags) -> Result<(FileHandle, (Inode, FileAttribute), FUSEOpenResponseFlags), PosixError> {
        let mut fs = self.fs.lock().unwrap();
        let new_inode = fs.next_inode.clone();
        if let Some(parent_node) = fs.inodes.get_mut(&parent) {
            let attr = FileAttribute {
                size: 0,
                blocks: 1,
                atime: SystemTime::now(),
                mtime: SystemTime::now(),
                ctime: SystemTime::now(),
                crtime: SystemTime::now(),
                kind: FileKind::RegularFile,
                perm: mode as u16,
                nlink: 1,
                uid: req.uid,
                gid: req.gid,
                rdev: 0,
                flags: 0,
                blksize: 512,
                ttl: None,
                generation: None,
            };

            let new_node = FSNode {
                parent,
                attr: attr.clone(),
                data: Vec::new(),
                children: HashMap::new(),
            };

            parent_node.children.insert(name.to_owned(), new_inode.clone());
            fs.inodes.insert(new_inode.clone(), new_node);
            fs.next_inode = new_inode.next();

            Ok((FileHandle::from(0), (new_inode.clone(), attr), FUSEOpenResponseFlags::empty()))
        } else {
            Err(ErrorKind::FileNotFound.to_error(""))
        }
    }
}