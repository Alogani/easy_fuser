use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fmt::Display,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

use std::sync::{atomic::AtomicU64, RwLock};

use crate::types::*;

pub const ROOT_INODE: u64 = 1;

/// FileIdType can have two values:
/// - Inode: in which case the user shall provide its own unique inode (at least a valid one)
/// - PathBuf: in which the inode to path mapping will be done and cached automatically

pub trait FileIdType: Send + std::fmt::Debug + 'static {
    type Converter: FileIdResolver<Output = Self>;

    fn get_converter() -> Self::Converter;
    fn display(&self) -> impl Display;
}

impl FileIdType for Inode {
    type Converter = InodeResolver;

    fn get_converter() -> Self::Converter {
        InodeResolver::new()
    }

    fn display(&self) -> impl Display {
        format!("{:?}", self)
    }
}
impl FileIdType for PathBuf {
    type Converter = PathBufResolver;

    fn get_converter() -> Self::Converter {
        PathBufResolver::new()
    }

    fn display(&self) -> impl Display {
        Path::display(self)
    }
}

/// FileIdResolver
/// FileIdResolver handles its data behind Locks if needed and should not be nested inside a Mutex

pub trait FileIdResolver: Send + Sync + 'static {
    type Output: FileIdType;
    fn new() -> Self;
    fn resolve_id(&self, ino: u64) -> Self::Output;
    fn assign_or_initialize_ino(&self, ino: u64, child: Option<&OsStr>, new_inode: &mut Inode);
    fn rename(&self, parent: u64, name: &OsStr, newparent: u64, newname: OsString);
    fn unlink(&self, parent: u64, name: &OsStr);
}

pub struct InodeResolver {}

impl FileIdResolver for InodeResolver {
    type Output = Inode;
    fn new() -> Self {
        Self {}
    }

    fn resolve_id(&self, ino: u64) -> Self::Output {
        Inode::from(ino)
    }

    // Do nothing, user should provide its own inode
    fn assign_or_initialize_ino(&self, _ino: u64, _child: Option<&OsStr>, new_inode: &mut Inode) {
        if *new_inode == INVALID_INODE {
            panic!("Provided inode should be valid (non zero)")
        }
    }
    fn rename(&self, _parent: u64, _name: &OsStr, _newparent: u64, _newname: OsString) {}
    fn unlink(&self, _parent: u64, _name: &OsStr) {}
}

pub struct PathBufResolver {
    inodes: RwLock<HashMap<u64, InodeValue>>,
    next_inode: AtomicU64,
}

struct InodeValue {
    name_ptr: SendOsStrPtr,
    parent: u64,
    children: HashMap<OsString, u64>,
}

struct SendOsStrPtr(*const OsStr);
unsafe impl Send for SendOsStrPtr {}
unsafe impl Sync for SendOsStrPtr {}

impl SendOsStrPtr {
    fn from(ptr: &OsStr) -> Self {
        SendOsStrPtr(ptr as *const OsStr)
    }

    fn get(&self) -> &OsStr {
        unsafe { &*self.0 }
    }
}

impl FileIdResolver for PathBufResolver {
    type Output = PathBuf;

    fn new() -> Self {
        let mut inodes = HashMap::new();

        // No leading slahs make it easier for path joining
        static ROOT_PATH: &str = "";

        inodes.insert(
            ROOT_INODE,
            InodeValue {
                name_ptr: SendOsStrPtr::from(unsafe {
                    &*(ROOT_PATH as *const str as *const OsStr)
                }),
                parent: ROOT_INODE,
                children: HashMap::new(),
            },
        );

        Self {
            inodes: RwLock::new(inodes),
            next_inode: AtomicU64::new(2), // Start assigning inodes from 2
        }
    }

    fn resolve_id(&self, ino: u64) -> Self::Output {
        let inodes = self.inodes.read().unwrap();
        let mut result = PathBuf::new();
        let mut inode = ino;
        loop {
            let (parent, name) = {
                let InodeValue {
                    name_ptr,
                    parent,
                    children: _,
                } = inodes.get(&inode).unwrap();
                (*parent, name_ptr.get().as_ref() as &Path)
            };
            if !result.as_os_str().is_empty() {
                result = name.join(result);
            } else {
                result = name.to_path_buf();
            }
            inode = parent;
            if inode == ROOT_INODE {
                break;
            }
        }
        result
    }

    fn assign_or_initialize_ino(&self, ino: u64, child: Option<&OsStr>, new_inode: &mut Inode) {
        let mut inodes = self.inodes.write().unwrap();
        let correct_inode = match child {
            None => ino,
            Some(child_name) if child_name == "." => ino,
            Some(child_name) if child_name == ".." => inodes.get(&ino).unwrap().parent,
            Some(child_name) => match inodes.get(&ino).unwrap().children.get(child_name) {
                Some(child_inode) => *child_inode,
                None => {
                    let next_inode = self.next_inode.fetch_add(1, Ordering::SeqCst);
                    let child_name = child_name.to_os_string();
                    let child_ptr = SendOsStrPtr::from(&child_name);
                    inodes
                        .get_mut(&ino)
                        .unwrap()
                        .children
                        .insert(child_name, next_inode);
                    inodes.insert(
                        next_inode,
                        InodeValue {
                            name_ptr: child_ptr,
                            parent: ino,
                            children: HashMap::new(),
                        },
                    );
                    next_inode
                }
            },
        };
        *new_inode = correct_inode.into();
    }

    fn rename(&self, parent: u64, name: &OsStr, newparent: u64, newname: OsString) {
        let mut inodes = self.inodes.write().unwrap();
        let new_name_ptr = SendOsStrPtr::from(&newname);
        let src_inode = match inodes.get_mut(&parent).unwrap().children.remove(name) {
            Some(inode) => {
                inodes.get_mut(&inode).unwrap().name_ptr = new_name_ptr;
                inode
            }
            None => {
                let new_inode = self.next_inode.fetch_add(1, Ordering::SeqCst);
                inodes.insert(
                    new_inode,
                    InodeValue {
                        name_ptr: new_name_ptr,
                        parent,
                        children: HashMap::new(),
                    },
                );
                new_inode
            }
        };
        inodes
            .get_mut(&newparent)
            .unwrap()
            .children
            .insert(newname, src_inode);
    }

    fn unlink(&self, parent: u64, name: &OsStr) {
        let mut inodes = self.inodes.write().unwrap();
        if let Some(inode) = inodes.get_mut(&parent).unwrap().children.remove(name) {
            let _ = inodes.remove(&inode);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;

    fn new_default_attr() -> FileAttribute {
        let now = SystemTime::now();
        FileAttribute {
            inode: Inode::from(0),
            size: 0,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: FileType::RegularFile,
            perm: 0,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 0,
            flags: 0,
            ttl: None,
            generation: None,
        }
    }

    #[test]
    fn test_new() {
        let converter = PathBufResolver::new();
        assert!(converter.inodes.read().unwrap().contains_key(&ROOT_INODE));
        assert_eq!(converter.next_inode.load(Ordering::SeqCst), 2);
        let inodes = converter.inodes.read().unwrap();
        let root_inode = inodes.get(&ROOT_INODE).unwrap();
        assert_eq!(root_inode.parent, ROOT_INODE);
        assert_eq!(root_inode.children.len(), 0);
        assert_eq!(root_inode.name_ptr.get(), "");
    }

    #[test]
    fn test_resolve_id() {
        let converter = PathBufResolver::new();
        let mut shallow_attr = new_default_attr();
        let mut nested_attr = new_default_attr();

        // Map shallow and nested paths
        converter.assign_or_initialize_ino(
            ROOT_INODE,
            Some(OsStr::new("shallow_file")),
            &mut shallow_attr.inode,
        );
        converter.assign_or_initialize_ino(
            shallow_attr.inode.clone().into(),
            Some(OsStr::new("nested_file")),
            &mut nested_attr.inode,
        );

        // Test shallow path
        let shallow_path = converter.resolve_id(shallow_attr.inode.into());
        assert_eq!(shallow_path, PathBuf::from("shallow_file"));

        // Test nested path
        let nested_path = converter.resolve_id(nested_attr.inode.into());
        assert_eq!(nested_path, PathBuf::from("shallow_file/nested_file"));
    }

    #[test]
    fn test_get_or_create_inode() {
        let converter = PathBufResolver::new();
        let mut shallow_attr = new_default_attr();
        let mut nested_attr = new_default_attr();

        // Map shallow and nested paths
        converter.assign_or_initialize_ino(
            ROOT_INODE,
            Some(OsStr::new("dir")),
            &mut shallow_attr.inode,
        );
        let shallow_ino = shallow_attr.inode.clone().into();
        converter.assign_or_initialize_ino(
            shallow_ino,
            Some(OsStr::new("file")),
            &mut nested_attr.inode,
        );

        // Get inodes
        let inodes = converter.inodes.read().unwrap();
        // Verify shallow path
        assert_eq!(shallow_ino, 2);
        assert!(inodes.contains_key(&shallow_ino));
        let shallow_inode = inodes.get(&shallow_ino).unwrap();
        assert_eq!(shallow_inode.parent, ROOT_INODE);
        assert_eq!(shallow_inode.name_ptr.get(), "dir");

        // Verify nested path
        let nested_attr_ino = nested_attr.inode.clone().into();
        assert!(inodes.contains_key(&nested_attr_ino));
        let nested_inode = inodes.get(&nested_attr_ino).unwrap();
        assert_eq!(nested_inode.parent, shallow_attr.inode.into());
        assert_eq!(nested_inode.name_ptr.get(), "file");
    }

    #[test]
    fn test_rename() {
        let converter = PathBufResolver::new();
        let mut shallow_attr = new_default_attr();
        let mut nested_attr = new_default_attr();

        // Map shallow and nested paths
        converter.assign_or_initialize_ino(
            ROOT_INODE,
            Some(OsStr::new("dir")),
            &mut shallow_attr.inode,
        );
        let shallow_ino = shallow_attr.inode.clone().into();
        converter.assign_or_initialize_ino(
            shallow_ino,
            Some(OsStr::new("file")),
            &mut nested_attr.inode,
        );

        // Rename shallow path
        converter.rename(
            ROOT_INODE,
            OsStr::new("dir"),
            ROOT_INODE,
            OsString::from("new_dir"),
        );
        let renamed_shallow_path = converter.resolve_id(shallow_ino);
        assert_eq!(renamed_shallow_path, PathBuf::from("new_dir"));

        // Verify nested path after rename
        let renamed_nested_path = converter.resolve_id(nested_attr.inode.into());
        assert_eq!(renamed_nested_path, PathBuf::from("new_dir/file"));
    }

    #[test]
    fn test_remove() {
        let converter = PathBufResolver::new();
        let mut shallow_attr = new_default_attr();
        let mut nested_attr = new_default_attr();

        // Map shallow and nested paths
        converter.assign_or_initialize_ino(
            ROOT_INODE,
            Some(OsStr::new("dir")),
            &mut shallow_attr.inode,
        );
        let shallow_ino = shallow_attr.inode.clone().into();
        converter.assign_or_initialize_ino(
            shallow_ino,
            Some(OsStr::new("file")),
            &mut nested_attr.inode,
        );

        // Remove nested path
        converter.unlink(shallow_ino, OsStr::new("file"));
        {
            let inodes = converter.inodes.read().unwrap();
            assert!(!inodes.contains_key(&nested_attr.inode.into()));
            let shallow_inode = inodes.get(&shallow_ino).unwrap();
            assert!(!shallow_inode
                .children
                .contains_key("file".as_ref() as &OsStr));
        }

        // Remove shallow path
        converter.unlink(ROOT_INODE, OsStr::new("dir"));
        {
            let inodes = converter.inodes.read().unwrap();
            assert!(!inodes.contains_key(&shallow_attr.inode.into()));
            let root_inode = inodes.get(&ROOT_INODE).unwrap();
            assert!(!root_inode.children.contains_key("dir".as_ref() as &OsStr));
        }
    }
}
