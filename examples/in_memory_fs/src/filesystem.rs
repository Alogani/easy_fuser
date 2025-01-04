use easy_fuser::prelude::*;
use easy_fuser::templates::DefaultFuseHandler;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Structure principale du système de fichiers en mémoire
pub struct InMemoryFS {
    inner: DefaultFuseHandler,
    fs: Arc<Mutex<DataBank>>, // Données partagées et protégées par un mutex
}

/// Représentation interne du système de fichiers
struct DataBank {
    inodes: HashMap<Inode, FSNode>, // Stockage des nœuds du système de fichiers
    next_inode: Inode,              // Prochain inode disponible
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
                    perm: 0o777, // To allow user to create files and directories
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

    // Access is not called for every operation
    fn access(&self, req: &RequestInfo, file_id: Inode, mask: AccessMask) -> FuseResult<()> {
        let fs = self.fs.lock().unwrap();
        let node = fs.inodes.get(&file_id)
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("File not found"))?;

        let file_mode = node.attr.perm;
        let file_uid = node.attr.uid;
        let file_gid = node.attr.gid;
    
        // Check if the user is root (uid 0)
        if req.uid == 0 {
            return Ok(());
        }
    
        let mut allowed_mask = AccessMask::empty();
    
        // Owner permissions
        if req.uid == file_uid {
            if file_mode & 0o400 != 0 { allowed_mask |= AccessMask::CAN_READ; }
            if file_mode & 0o200 != 0 { allowed_mask |= AccessMask::CAN_WRITE; }
            if file_mode & 0o100 != 0 { allowed_mask |= AccessMask::CAN_EXEC; }
        } 
        // Group permissions
        else if req.gid == file_gid {
            if file_mode & 0o040 != 0 { allowed_mask |= AccessMask::CAN_READ; }
            if file_mode & 0o020 != 0 { allowed_mask |= AccessMask::CAN_WRITE; }
            if file_mode & 0o010 != 0 { allowed_mask |= AccessMask::CAN_EXEC; }
        } 
        // Others permissions
        else {
            if file_mode & 0o004 != 0 { allowed_mask |= AccessMask::CAN_READ; }
            if file_mode & 0o002 != 0 { allowed_mask |= AccessMask::CAN_WRITE; }
            if file_mode & 0o001 != 0 { allowed_mask |= AccessMask::CAN_EXEC; }
        }
    
        // Special cases for directories
        if node.attr.kind == FileKind::Directory {
            // Always need execute permission to access a directory
            if !allowed_mask.contains(AccessMask::CAN_EXEC) {
                return Err(ErrorKind::PermissionDenied.to_error("Execute permission required for directory"));
            }
            // Writing to a directory means adding or removing entries, which requires write permission
            if mask.contains(AccessMask::CAN_WRITE) && !allowed_mask.contains(AccessMask::CAN_WRITE) {
                return Err(ErrorKind::PermissionDenied.to_error("Write permission required for directory modification"));
            }
        }
    
        if allowed_mask.contains(mask) {
            Ok(())
        } else {
            Err(ErrorKind::PermissionDenied.to_error("Permission denied"))
        }
    }

    fn create(&self, req: &RequestInfo, parent: Inode, name: &OsStr, mode: u32, _umask: u32, _flags: OpenFlags) -> Result<(FileHandle, (Inode, FileAttribute), FUSEOpenResponseFlags), PosixError> {
        self.access(req, parent.clone(), AccessMask::CAN_WRITE)?;
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

    fn fallocate(
        &self,
        req: &RequestInfo,
        file_id: Inode,
        _file_handle: FileHandle,
        offset: i64,
        length: i64,
        mode: FallocateFlags,
    ) -> FuseResult<()> {
        self.access(req, file_id.clone(), AccessMask::CAN_WRITE)?;
        let mut fs = self.fs.lock().unwrap();
    
        if let Some(node) = fs.inodes.get_mut(&file_id) {
            if node.attr.kind != FileKind::RegularFile {
                return Err(ErrorKind::InvalidArgument.to_error("Not a regular file"));
            }
    
            let offset = offset as usize;
            let length = length as usize;
    
            // Check if we need to allocate or deallocate
            if mode.contains(FallocateFlags::PUNCH_HOLE) {
                // Punch a hole (deallocate)
                if offset + length > node.data.len() {
                    return Err(ErrorKind::InvalidArgument.to_error("Invalid range"));
                }
                node.data[offset..offset + length].fill(0);
            } else {
                // Allocate space
                let new_size = std::cmp::max(node.data.len(), offset + length);
                node.data.resize(new_size, 0);
            }
    
            // Update file attributes
            node.attr.size = node.data.len() as u64;
            node.attr.blocks = (node.data.len() as u64 + 511) / 512; // Round up to nearest block
            node.attr.mtime = SystemTime::now();
            node.attr.ctime = SystemTime::now();
    
            Ok(())
        } else {
            Err(ErrorKind::FileNotFound.to_error(""))
        }
    }

    fn flush(
            &self,
            _req: &RequestInfo,
            _file_id: Inode,
            _file_handle: FileHandle,
            _lock_owner: u64,
        ) -> FuseResult<()> {
        Ok(())
    }

    fn fsync(
            &self,
            _req: &RequestInfo,
            _file_id: Inode,
            _file_handle: FileHandle,
            _datasync: bool,
        ) -> FuseResult<()> {
        Ok(())
    }

    fn getattr(&self, _req: &RequestInfo, ino: Inode, _fh: Option<FileHandle>) -> FuseResult<FileAttribute> {
        let fs = self.fs.lock().unwrap();
        fs.inodes.get(&ino)
            .map(|node| node.attr.clone())
            .ok_or_else(|| ErrorKind::FileNotFound.to_error(""))
    }

    fn lookup(&self, req: &RequestInfo, parent: Inode, name: &OsStr) -> FuseResult<(Inode, FileAttribute)> {
        self.access(req, parent.clone(), AccessMask::CAN_READ)?;
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

    fn mkdir(&self, req: &RequestInfo, parent: Inode, name: &OsStr, mode: u32, _umask: u32) -> FuseResult<(Inode, FileAttribute)> {
        self.access(req, parent.clone(), AccessMask::CAN_WRITE)?;
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

    fn read(&self, req: &RequestInfo, ino: Inode, _fh: FileHandle, offset: SeekFrom, size: u32, _flags: FUSEOpenFlags, _lock_owner: Option<u64>) -> FuseResult<Vec<u8>> {
        self.access(req, ino.clone(), AccessMask::CAN_READ)?;
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

    fn readdir(&self, req: &RequestInfo, ino: Inode, _fh: FileHandle) -> FuseResult<Vec<(OsString, (Inode, FileKind))>> {
        self.access(req, ino.clone(), AccessMask::CAN_READ)?;
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

    fn rename(
        &self,
        req: &RequestInfo,
        parent_id: Inode,
        name: &OsStr,
        newparent: Inode,
        newname: &OsStr,
        flags: RenameFlags,
    ) -> FuseResult<()> {
        self.access(req, parent_id.clone(), AccessMask::CAN_WRITE)?;
        self.access(req, newparent.clone(), AccessMask::CAN_WRITE)?;
        let mut fs = self.fs.lock().unwrap();
        
        // Check if the source exists and get its inode
        let source_inode = fs.inodes.get(&parent_id)
            .and_then(|parent| parent.children.get(name))
            .cloned()
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("Source not found"))?;

        // Check if the destination already exists
        let existing_dest = fs.inodes.get(&newparent)
            .and_then(|parent| parent.children.get(newname).cloned());

        if let Some(existing_dest) = existing_dest {
            if flags.contains(RenameFlags::NOREPLACE) {
                return Err(ErrorKind::FileExists.to_error("Destination already exists"));
            }
            // If REPLACE flag is set or no flags, remove the existing destination
            fs.inodes.remove(&existing_dest);
        }

        // Remove the source from its parent
        if let Some(source_parent) = fs.inodes.get_mut(&parent_id) {
            source_parent.children.remove(name);
        } else {
            return Err(ErrorKind::FileNotFound.to_error("Source parent not found"));
        }

        // Add the source to its new parent
        if let Some(dest_parent) = fs.inodes.get_mut(&newparent) {
            dest_parent.children.insert(newname.to_owned(), source_inode.clone());
        } else {
            return Err(ErrorKind::FileNotFound.to_error("Destination parent not found"));
        }

        // Update the parent of the renamed node
        if let Some(node) = fs.inodes.get_mut(&source_inode) {
            node.parent = newparent;
            node.attr.ctime = SystemTime::now();
        }
        Ok(())
    }

    fn rmdir(&self, req: &RequestInfo, parent_id: Inode, name: &OsStr) -> FuseResult<()> {
        self.access(req, parent_id.clone(), AccessMask::CAN_WRITE)?;
        let mut fs = self.fs.lock().unwrap();
        
        // Check if the dir to remove exists and get its inode
        let child_inode = fs.inodes.get(&parent_id)
            .and_then(|parent| parent.children.get(name))
            .cloned()
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("Source not found"))?;
        
        {
            let child = fs.inodes.get(&child_inode).unwrap();
            if child.attr.kind != FileKind::Directory {
                return Err(ErrorKind::NotADirectory.to_error("Not a directory"));
            };
            if child.children.len() > 0 {
                return Err(ErrorKind::DirectoryNotEmpty.to_error("Directory not empty"));
            };
        }
                
        // Remove the directory inode
        fs.inodes.remove(&child_inode);

        // Remove the directory from its parent
        let parent = fs.inodes.get_mut(&parent_id).unwrap();
        parent.children.remove(name);
        
        // Update parent's mtime and ctime
        parent.attr.mtime = SystemTime::now();
        parent.attr.ctime = SystemTime::now();
        
        Ok(())
    }
    
    fn setattr(
        &self,
        req: &RequestInfo,
        file_id: Inode,
        attrs: SetAttrRequest,
    ) -> FuseResult<FileAttribute> {
        let mut fs = self.fs.lock().unwrap();
        
        if let Some(node) = fs.inodes.get_mut(&file_id) {
            // Simplification of rights management, assuming only owner can modify attributes
            if node.attr.uid != req.uid {
                return Err(ErrorKind::PermissionDenied.to_error("Only the owner can modify attributes"));
            }

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

    fn write(&self, req: &RequestInfo, ino: Inode, _fh: FileHandle, offset: SeekFrom, data: Vec<u8>, _write_flags: FUSEWriteFlags, _flags: OpenFlags, _lock_owner: Option<u64>) -> FuseResult<u32> {
        self.access(req, ino.clone(), AccessMask::CAN_WRITE)?;
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

    fn unlink(&self, req: &RequestInfo, parent_id: Inode, name: &OsStr) -> FuseResult<()> {
        self.access(req, parent_id.clone(), AccessMask::CAN_WRITE)?;
        let mut fs = self.fs.lock().unwrap();

        if let Some(child_inode) = fs.inodes
            .get_mut(&parent_id)
            .unwrap()
            .children.remove(name)
        {
            fs.inodes.remove(&child_inode);
            Ok(())
        } else {
            return Err(ErrorKind::FileNotFound.to_error("File not found"));
        }
    }
}