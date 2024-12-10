use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use crate::types::*;

pub const ROOT_INODE: u64 = 1;

pub trait IdType: Send {}

impl IdType for Inode {}
impl IdType for PathBuf {}

pub trait IdConverter: Send + 'static {
    type Output: IdType;
    fn new() -> Self;
    fn to_id(&self, ino: u64) -> Self::Output;
    fn map_inode(&mut self, ino: u64, child: Option<&OsStr>, new_inode: &mut Inode);
    fn rename(&mut self, parent: u64, name: &OsStr, newparent: u64, newname: OsString);
    fn remove(&mut self, parent: u64, name: &OsStr);
}

pub struct InoToInode {}

impl IdConverter for InoToInode {
    type Output = Inode;
    fn new() -> Self {
        Self {}
    }

    fn to_id(&self, ino: u64) -> Self::Output {
        Inode::from(ino)
    }

    // Do nothing, user should provide its own inode
    fn map_inode(&mut self, _ino: u64, _child: Option<&OsStr>, _new_inode: &mut Inode) {}
    fn rename(&mut self, _parent: u64, _name: &OsStr, _newparent: u64, _newname: OsString) {}
    fn remove(&mut self, _parent: u64, _name: &OsStr) {}
}

struct InoToPath {
    inodes: HashMap<u64, InodeValue>,
    next_inode: u64,
}

struct InodeValue {
    name_ptr: SendOsStrPtr,
    parent: u64,
    children: HashMap<OsString, u64>,
}

struct SendOsStrPtr(*const OsStr);

unsafe impl Send for SendOsStrPtr {}

impl SendOsStrPtr {
    fn from(ptr: &OsStr) -> Self {
        SendOsStrPtr(ptr as *const OsStr)
    }

    fn get(&self) -> &OsStr {
        unsafe { &*self.0 }
    }
}

impl IdConverter for InoToPath {
    type Output = PathBuf;

    fn new() -> Self {
        let mut inodes = HashMap::new();

        static ROOT_PATH: &str = "/";

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
            inodes,
            next_inode: 2, // Start assigning inodes from 2
        }
    }

    fn to_id(&self, ino: u64) -> Self::Output {
        let mut result = PathBuf::new();
        let mut inode = ino;
        loop {
            let (parent, name) = {
                let InodeValue {
                    name_ptr,
                    parent,
                    children: _,
                } = self.inodes.get(&inode).unwrap();
                (*parent, name_ptr.get().as_ref() as &Path)
            };
            if !result.as_os_str().is_empty() {
                result = name.join(result);
            } else {
                result = name.to_path_buf();
            }
            if inode == ROOT_INODE {
                break;
            }
            inode = parent;
        }
        result
    }

    fn map_inode(&mut self, ino: u64, child: Option<&OsStr>, new_inode: &mut Inode) {
        let correct_inode = match child {
            None => ino,
            Some(child_name) => match self.inodes.get(&ino).unwrap().children.get(child_name) {
                Some(child_inode) => *child_inode,
                None => {
                    let next_inode = self.next_inode;
                    self.next_inode += 1;
                    let child_name = child_name.to_os_string();
                    let child_ptr = SendOsStrPtr::from(&child_name);
                    self.inodes
                        .get_mut(&ino)
                        .unwrap()
                        .children
                        .insert(child_name, next_inode);
                    self.inodes.insert(
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

    fn rename(&mut self, parent: u64, name: &OsStr, newparent: u64, newname: OsString) {
        let new_name_ptr = SendOsStrPtr::from(&newname);
        let src_inode = match self.inodes.get_mut(&parent).unwrap().children.remove(name) {
            Some(inode) => {
                self.inodes.get_mut(&inode).unwrap().name_ptr = new_name_ptr;
                inode
            }
            None => {
                let new_inode = self.next_inode;
                self.next_inode += 1;
                self.inodes.insert(
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
        self.inodes
            .get_mut(&newparent)
            .unwrap()
            .children
            .insert(newname, src_inode);
    }

    fn remove(&mut self, parent: u64, name: &OsStr) {
        if let Some(inode) = self.inodes.get_mut(&parent).unwrap().children.remove(name) {
            let _ = self.inodes.remove(&inode);
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
        let converter = InoToPath::new();
        assert!(converter.inodes.contains_key(&ROOT_INODE));
        assert_eq!(converter.next_inode, 2);
        let root_inode = converter.inodes.get(&ROOT_INODE).unwrap();
        assert_eq!(root_inode.parent, ROOT_INODE);
        assert_eq!(root_inode.children.len(), 0);
        assert_eq!(root_inode.name_ptr.get(), "/");
    }

    #[test]
    fn test_to_id() {
        let mut converter = InoToPath::new();
        let mut shallow_attr = new_default_attr();
        let mut nested_attr = new_default_attr();

        // Map shallow and nested paths
        converter.map_inode(
            ROOT_INODE,
            Some(OsStr::new("shallow_file")),
            &mut shallow_attr.inode,
        );
        converter.map_inode(
            shallow_attr.inode.clone().into(),
            Some(OsStr::new("nested_file")),
            &mut nested_attr.inode,
        );

        // Test shallow path
        let shallow_path = converter.to_id(shallow_attr.inode.into());
        assert_eq!(shallow_path, PathBuf::from("/shallow_file"));

        // Test nested path
        let nested_path = converter.to_id(nested_attr.inode.into());
        assert_eq!(nested_path, PathBuf::from("/shallow_file/nested_file"));
    }

    #[test]
    fn test_map_inode() {
        let mut converter = InoToPath::new();
        let mut shallow_attr = new_default_attr();
        let mut nested_attr = new_default_attr();

        // Map shallow and nested paths
        converter.map_inode(ROOT_INODE, Some(OsStr::new("dir")), &mut shallow_attr.inode);
        let shallow_ino = shallow_attr.inode.clone().into();
        converter.map_inode(
            shallow_ino,
            Some(OsStr::new("file")),
            &mut nested_attr.inode,
        );

        // Verify shallow path
        assert_eq!(shallow_ino, 2);
        assert!(converter.inodes.contains_key(&shallow_ino));
        let shallow_inode = converter.inodes.get(&shallow_ino).unwrap();
        assert_eq!(shallow_inode.parent, ROOT_INODE);
        assert_eq!(shallow_inode.name_ptr.get(), "dir");

        // Verify nested path
        let nested_attr_ino = nested_attr.inode.clone().into();
        assert!(converter.inodes.contains_key(&nested_attr_ino));
        let nested_inode = converter.inodes.get(&nested_attr_ino).unwrap();
        assert_eq!(nested_inode.parent, shallow_attr.inode.into());
        assert_eq!(nested_inode.name_ptr.get(), "file");
    }

    #[test]
    fn test_rename() {
        let mut converter = InoToPath::new();
        let mut shallow_attr = new_default_attr();
        let mut nested_attr = new_default_attr();

        // Map shallow and nested paths
        converter.map_inode(ROOT_INODE, Some(OsStr::new("dir")), &mut shallow_attr.inode);
        let shallow_ino = shallow_attr.inode.clone().into();
        converter.map_inode(
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
        let renamed_shallow_path = converter.to_id(shallow_ino);
        assert_eq!(renamed_shallow_path, PathBuf::from("/new_dir"));

        // Verify nested path after rename
        let renamed_nested_path = converter.to_id(nested_attr.inode.into());
        assert_eq!(renamed_nested_path, PathBuf::from("/new_dir/file"));
    }

    #[test]
    fn test_remove() {
        let mut converter = InoToPath::new();
        let mut shallow_attr = new_default_attr();
        let mut nested_attr = new_default_attr();

        // Map shallow and nested paths
        converter.map_inode(ROOT_INODE, Some(OsStr::new("dir")), &mut shallow_attr.inode);
        let shallow_ino = shallow_attr.inode.clone().into();
        converter.map_inode(
            shallow_ino,
            Some(OsStr::new("file")),
            &mut nested_attr.inode,
        );

        // Remove nested path
        converter.remove(shallow_ino, OsStr::new("file"));
        assert!(!converter.inodes.contains_key(&nested_attr.inode.into()));
        let shallow_inode = converter.inodes.get(&shallow_ino).unwrap();
        assert!(!shallow_inode
            .children
            .contains_key("file".as_ref() as &OsStr));

        // Remove shallow path
        converter.remove(ROOT_INODE, OsStr::new("dir"));
        assert!(!converter.inodes.contains_key(&shallow_attr.inode.into()));
        let root_inode = converter.inodes.get(&ROOT_INODE).unwrap();
        assert!(!root_inode.children.contains_key("dir".as_ref() as &OsStr));
    }
}
