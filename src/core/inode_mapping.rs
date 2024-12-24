use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

use std::sync::{atomic::AtomicU64, RwLock};

use crate::types::*;

pub const ROOT_INODE: u64 = 1;

pub trait GetConverter {
    type Resolver: FileIdResolver<FileIdType = Self>;
    fn get_converter() -> Self::Resolver;
}

impl GetConverter for Inode {
    type Resolver = InodeResolver;
    fn get_converter() -> Self::Resolver {
        InodeResolver::new()
    }
}

impl GetConverter for PathBuf {
    type Resolver = PathBufResolver;
    fn get_converter() -> Self::Resolver {
        PathBufResolver::new()
    }
}

/// FileIdResolver
/// FileIdResolver handles its data behind Locks if needed and should not be nested inside a Mutex

pub trait FileIdResolver: Send + Sync + 'static {
    type FileIdType: FileIdType;

    fn new() -> Self;
    fn resolve_id(&self, ino: u64) -> Self::FileIdType;
    fn lookup(
        &self,
        parent: u64,
        child: &OsStr,
        id: <Self::FileIdType as FileIdType>::_Id,
        increment: bool,
    ) -> u64;
    fn add_children(
        &self,
        parent: u64,
        children: Vec<(OsString, <Self::FileIdType as FileIdType>::_Id)>,
        increment: bool,
    ) -> Vec<(OsString, u64)>;
    fn forget(&self, ino: u64, nlookup: u64);
    fn rename(&self, parent: u64, name: &OsStr, newparent: u64, newname: &OsStr);
}

pub struct InodeResolver {}

impl FileIdResolver for InodeResolver {
    type FileIdType = Inode;

    fn new() -> Self {
        Self {}
    }

    fn resolve_id(&self, ino: u64) -> Self::FileIdType {
        Inode::from(ino)
    }

    fn lookup(&self, _parent: u64, _child: &OsStr, id: Inode, _increment: bool) -> u64 {
        id.into()
    }

    // Do nothing, user should provide its own inode
    fn add_children(
        &self,
        parent: u64,
        children: Vec<(OsString, Inode)>,
        increment: bool,
    ) -> Vec<(OsString, u64)> {
        children
            .into_iter()
            .map(|(name, inode)| (name, u64::from(inode)))
            .collect()
    }

    fn forget(&self, _ino: u64, _nlookup: u64) {}

    fn rename(&self, _parent: u64, _name: &OsStr, _newparent: u64, _newname: &OsStr) {}
}

pub struct PathBufResolver {
    inodes: RwLock<HashMap<u64, InodeValue>>,
    next_inode: AtomicU64,
}

struct InodeValue {
    name_ptr: SendOsStrPtr,
    nlookup: u64,
    parent: u64,
    children: HashMap<OsString, u64>,
}

#[derive(Clone)]
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
    type FileIdType = PathBuf;

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
                nlookup: 0, // not used, root is never forget
                parent: ROOT_INODE,
                children: HashMap::new(),
            },
        );

        Self {
            inodes: RwLock::new(inodes),
            next_inode: AtomicU64::new(2), // Start assigning inodes from 2
        }
    }

    fn resolve_id(&self, ino: u64) -> Self::FileIdType {
        let inodes = self.inodes.read().unwrap();
        let mut result = PathBuf::new();
        let mut inode = ino;
        loop {
            let (parent, name) = {
                let InodeValue {
                    name_ptr,
                    nlookup: _,
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
        drop(inodes);
        result
    }

    fn lookup(&self, parent: u64, child: &OsStr, _id: (), increment: bool) -> u64 {
        let mut inodes = self.inodes.write().unwrap();
        let correct_inode = match child {
            os_str if os_str == "." => parent,
            os_str if os_str == ".." => inodes.get(&parent).unwrap().parent,
            _ => match inodes.get(&parent).unwrap().children.get(child) {
                Some(&child_inode) => {
                    if increment {
                        inodes.get_mut(&child_inode).unwrap().nlookup += 1;
                    }
                    child_inode
                }
                None => {
                    let next_inode = self.next_inode.fetch_add(1, Ordering::SeqCst);
                    let child_name = child.to_os_string();
                    let child_ptr = SendOsStrPtr::from(&child_name);
                    inodes
                        .get_mut(&parent)
                        .unwrap()
                        .children
                        .insert(child_name, next_inode);
                    inodes.insert(
                        next_inode,
                        InodeValue {
                            name_ptr: child_ptr,
                            nlookup: if increment { 1 } else { 0 },
                            parent,
                            children: HashMap::new(),
                        },
                    );
                    next_inode
                }
            },
        };
        correct_inode
    }

    fn add_children(
        &self,
        parent: u64,
        children: Vec<(OsString, ())>,
        increment: bool,
    ) -> Vec<(OsString, u64)> {
        let mut inodes = self.inodes.write().unwrap();
        if inodes.get(&parent).unwrap().children.is_empty() {
            let mut result = Vec::new();
            for (child_name, _) in children {
                let child_inode = match child_name.as_os_str() {
                    child_name if child_name == "." => parent,
                    child_name if child_name == ".." => inodes.get(&parent).unwrap().parent,
                    _ => {
                        let next_inode = self.next_inode.fetch_add(1, Ordering::SeqCst);
                        let child_ptr = SendOsStrPtr::from(&child_name);
                        inodes
                            .get_mut(&parent)
                            .unwrap()
                            .children
                            .insert(child_name.clone(), next_inode);
                        inodes.insert(
                            next_inode,
                            InodeValue {
                                name_ptr: child_ptr,
                                nlookup: if increment { 1 } else { 0 },
                                parent,
                                children: HashMap::new(),
                            },
                        );
                        next_inode
                    }
                };
                result.push((child_name, child_inode));
            }
            result
        } else {
            children
                .into_iter()
                .map(|(child_name, metadata)| {
                    let child_inode = self.lookup(parent, &child_name, metadata, increment);
                    (child_name, child_inode)
                })
                .collect()
        }
    }

    fn forget(&self, ino: u64, nlookup: u64) {
        let mut inodes = self.inodes.write().unwrap();
        let inode_value = inodes.get_mut(&ino).unwrap();

        if inode_value.nlookup == nlookup {
            let parent = inode_value.parent;
            let name_ptr = inode_value.name_ptr.clone();
            inodes
                .get_mut(&parent)
                .unwrap()
                .children
                .remove(name_ptr.get());
            inodes.remove(&ino);
        } else {
            inode_value.nlookup -= nlookup;
        }
    }

    fn rename(&self, parent: u64, name: &OsStr, newparent: u64, newname: &OsStr) {
        let mut inodes = self.inodes.write().unwrap();
        let new_name_ptr = SendOsStrPtr::from(newname);
        let src_inode = inodes
            .get_mut(&parent)
            .unwrap()
            .children
            .remove(name)
            .unwrap();
        inodes.get_mut(&src_inode).unwrap().name_ptr = new_name_ptr;
        inodes
            .get_mut(&newparent)
            .unwrap()
            .children
            .insert(newname.to_owned(), src_inode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // Map shallow and nested paths
        let shallow_ino = converter.lookup(ROOT_INODE, OsStr::new("shallow_file"), (), true);
        let nested_ino = converter.lookup(shallow_ino, OsStr::new("nested_file"), (), true);

        // Test shallow path
        let shallow_path = converter.resolve_id(shallow_ino);
        assert_eq!(shallow_path, PathBuf::from("shallow_file"));

        // Test nested path
        let nested_path = converter.resolve_id(nested_ino);
        assert_eq!(nested_path, PathBuf::from("shallow_file/nested_file"));
    }

    #[test]
    fn test_get_or_create_inode() {
        let converter = PathBufResolver::new();

        // Map shallow and nested paths
        let shallow_ino = converter.lookup(ROOT_INODE, OsStr::new("dir"), (), true);
        let nested_ino = converter.lookup(shallow_ino, OsStr::new("file"), (), true);

        // Get inodes
        let inodes = converter.inodes.read().unwrap();
        // Verify shallow path
        assert_eq!(shallow_ino, 2);
        assert!(inodes.contains_key(&shallow_ino));
        let shallow_inode = inodes.get(&shallow_ino).unwrap();
        assert_eq!(shallow_inode.parent, ROOT_INODE);
        assert_eq!(shallow_inode.name_ptr.get(), "dir");

        // Verify nested path
        assert!(inodes.contains_key(&nested_ino));
        let nested_inode = inodes.get(&nested_ino).unwrap();
        assert_eq!(nested_inode.parent, shallow_ino);
        assert_eq!(nested_inode.name_ptr.get(), "file");
    }

    #[test]
    fn test_rename() {
        let converter = PathBufResolver::new();

        // Map shallow and nested paths
        let shallow_ino = converter.lookup(ROOT_INODE, OsStr::new("dir"), (), true);
        let nested_ino = converter.lookup(shallow_ino, OsStr::new("file"), (), true);

        // Rename shallow path
        converter.rename(
            ROOT_INODE,
            OsStr::new("dir"),
            ROOT_INODE,
            OsStr::new("new_dir"),
        );
        let renamed_shallow_path = converter.resolve_id(shallow_ino);
        assert_eq!(renamed_shallow_path, PathBuf::from("new_dir"));

        // Verify nested path after rename
        let renamed_nested_path = converter.resolve_id(nested_ino);
        assert_eq!(renamed_nested_path, PathBuf::from("new_dir/file"));
    }

    #[test]
    fn test_forget() {
        let converter = PathBufResolver::new();

        let shallow_ino = converter.lookup(ROOT_INODE, OsStr::new("dir"), (), true);
        let nested_ino = converter.lookup(shallow_ino, OsStr::new("file"), (), true);

        // Remove nested path
        converter.forget(nested_ino, 1);
        {
            let inodes = converter.inodes.read().unwrap();
            assert!(!inodes.contains_key(&nested_ino));
            let shallow_inode = inodes.get(&shallow_ino).unwrap();
            assert!(!shallow_inode
                .children
                .contains_key("file".as_ref() as &OsStr));
        }

        // Remove shallow path
        converter.forget(shallow_ino, 1);
        {
            let inodes = converter.inodes.read().unwrap();
            assert!(!inodes.contains_key(&shallow_ino));
            let root_inode = inodes.get(&ROOT_INODE).unwrap();
            assert!(!root_inode.children.contains_key("dir".as_ref() as &OsStr));
        }
    }
}
