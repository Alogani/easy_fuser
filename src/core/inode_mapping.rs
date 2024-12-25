use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::PathBuf,
    sync::atomic::Ordering,
};

use std::sync::{atomic::AtomicU64, RwLock};

use crate::{types::*, unwrap};

pub const ROOT_INODE: u64 = 1;

pub trait GetConverter {
    type Resolver: FileIdResolver<FileIdType = Self>;
    fn get_converter() -> Self::Resolver;
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

impl GetConverter for Inode {
    type Resolver = InodeResolver;
    fn get_converter() -> Self::Resolver {
        InodeResolver::new()
    }
}

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
        _parent: u64,
        children: Vec<(OsString, Inode)>,
        _increment: bool,
    ) -> Vec<(OsString, u64)> {
        children
            .into_iter()
            .map(|(name, inode)| (name, u64::from(inode)))
            .collect()
    }

    fn forget(&self, _ino: u64, _nlookup: u64) {}

    fn rename(&self, _parent: u64, _name: &OsStr, _newparent: u64, _newname: &OsStr) {}
}

pub struct ComponentsResolver {
    data: RwLock<InodeData>,
    next_inode: AtomicU64,
}

struct InodeData {
    inodes: HashMap<u64, InodeValue>,
    // Storing here instead of a HashMap for each children simplify the borrowing rules
    all_children: HashMap<u64, HashMap<OsString, u64>>,
}

impl InodeData {
    fn double_borrow(
        &mut self,
    ) -> (
        &mut HashMap<u64, InodeValue>,
        &mut HashMap<u64, HashMap<OsString, u64>>,
    ) {
        (&mut self.inodes, &mut self.all_children)
    }
}

struct InodeValue {
    nlookup: AtomicU64,
    parent: u64,
    name: OsString,
}

impl GetConverter for Vec<OsString> {
    type Resolver = ComponentsResolver;
    fn get_converter() -> Self::Resolver {
        ComponentsResolver::new()
    }
}

impl FileIdResolver for ComponentsResolver {
    type FileIdType = Vec<OsString>;

    fn new() -> Self {
        let mut inodes = HashMap::new();
        let mut all_children = HashMap::new();

        // Insert root inode
        inodes.insert(
            ROOT_INODE,
            InodeValue {
                nlookup: AtomicU64::new(0), // not used for root, never forgotten
                parent: ROOT_INODE,
                name: OsString::from(""),
            },
        );

        // Add empty children map for root inode
        all_children.insert(ROOT_INODE, HashMap::new());

        ComponentsResolver {
            data: RwLock::new(InodeData {
                inodes,
                all_children,
            }),
            next_inode: AtomicU64::new(ROOT_INODE + 1),
        }
    }

    fn resolve_id(&self, ino: u64) -> Self::FileIdType {
        let inodes = &self
            .data
            .read()
            .expect("Failed to acquire read lock")
            .inodes;
        let mut path_components = Vec::new();

        let mut current_ino = ino;

        while current_ino != 1 {
            let inode_value = unwrap!(inodes.get(&current_ino), "Inode not found: {:x?}", ino);
            path_components.push(inode_value.name.clone());
            current_ino = inode_value.parent;
        }
        
        path_components
    }

    fn lookup(&self, parent: u64, child: &OsStr, _id: (), increment: bool) -> u64 {
        {
            eprintln!("LOOKUP {:x?} - {:?}", parent, child);
            // Assume optimistic existence of child by acquiring only read lock
            let data = self.data.read().expect("Failed to acquire read lock");
            let children = unwrap!(
                data.all_children.get(&parent),
                "No such parent inode {:x?}",
                parent
            );
            if let Some(child_ino) = children.get(child) {
                if increment {
                    unwrap!(
                        data.inodes.get(child_ino),
                        "No such child inode {:x?}",
                        child_ino
                    )
                    .nlookup
                    .fetch_add(1, Ordering::SeqCst);
                }
                return *child_ino;
            }
        }
        let mut data = self.data.write().expect("Failed to acquire write lock");
        let new_inode = self.next_inode.fetch_add(1, Ordering::SeqCst);
        let children = unwrap!(
            data.all_children.get_mut(&parent),
            "No such parent inode {:x?}",
            parent
        );
        children.insert(child.to_os_string(), new_inode);
        data.inodes.insert(
            new_inode,
            InodeValue {
                nlookup: AtomicU64::new(if increment { 1 } else { 0 }),
                parent,
                name: child.to_os_string(),
            },
        );
        data.all_children.insert(new_inode, HashMap::new());
        new_inode
    }

    fn add_children(
        &self,
        parent: u64,
        children: Vec<(OsString, ())>,
        increment: bool,
    ) -> Vec<(OsString, u64)> {
        let mut data = self.data.write().expect("Failed to acquire write lock");
        let (inodes, all_children) = data.double_borrow();
        let children_len = children.len();
        let parent_children = unwrap!(
            all_children.get_mut(&parent),
            "No such parent inode {:x?}",
            parent
        );
        let mut new_inodes = Vec::with_capacity(children_len);
        let result = if parent_children.is_empty() {
            parent_children.reserve(children_len);
            let mut new_children = Vec::with_capacity(children_len);

            for (name, _) in children {
                let ino = self.next_inode.fetch_add(1, Ordering::SeqCst);
                parent_children.insert(name.clone(), ino);
                inodes.insert(
                    ino,
                    InodeValue {
                        nlookup: AtomicU64::new(if increment { 1 } else { 0 }),
                        parent,
                        name: name.clone(),
                    },
                );
                new_children.push((name, ino));
                new_inodes.push(ino);
            }

            new_children
        } else {
            parent_children.reserve(children_len - parent_children.len());
            let mut result = Vec::with_capacity(children.len());
            let mut new_inodes = Vec::with_capacity(children_len);

            for (name, _) in children {
                if let Some(&existing_ino) = parent_children.get(&name) {
                    // Child already exists, increment if necessary
                    if increment {
                        if let Some(inode_value) = inodes.get(&existing_ino) {
                            inode_value.nlookup.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    result.push((name, existing_ino));
                } else {
                    // Child doesn't exist, add it
                    let new_ino = self.next_inode.fetch_add(1, Ordering::SeqCst);
                    parent_children.insert(name.clone(), new_ino);
                    inodes.insert(
                        new_ino,
                        InodeValue {
                            nlookup: AtomicU64::new(if increment { 1 } else { 0 }),
                            parent,
                            name: name.clone(),
                        },
                    );
                    new_inodes.push(new_ino);
                    result.push((name, new_ino));
                }
            }
            result
        };

        // Insert new empty HashMaps for all new inodes
        for ino in new_inodes {
            all_children.insert(ino, HashMap::new());
        }
        result
    }

    fn forget(&self, ino: u64, nlookup: u64) {
        let mut data = self.data.write().expect("Failed to acquire write lock");
        let (inodes, all_children) = data.double_borrow();

        if let Some(inode_value) = inodes.get(&ino) {
            let new_nlookup = inode_value.nlookup.fetch_sub(nlookup, Ordering::SeqCst);
            if new_nlookup <= nlookup {
                // Remove the inode if its lookup count reaches zero or below
                let parent = inode_value.parent;
                let name = inode_value.name.clone();

                // Remove the inode from its parent's children
                if let Some(parent_children) = all_children.get_mut(&parent) {
                    parent_children.remove(&name);
                }

                // Remove the inode and its children
                inodes.remove(&ino);
                all_children.remove(&ino);
            }
        }
    }

    fn rename(&self, parent: u64, name: &OsStr, newparent: u64, newname: &OsStr) {
        let mut data = self.data.write().expect("Failed to acquire write lock");

        let children = unwrap!(
            data.all_children.get_mut(&parent),
            "No such parent inode {}",
            parent
        );
        let &ino = unwrap!(
            children.get(name),
            "Rename called on non existent child {:?} {:?}",
            parent,
            name
        );

        children.remove(name);

        let new_parent_children = unwrap!(
            data.all_children.get_mut(&newparent),
            "No such newparent inode {}",
            parent
        );
        new_parent_children.insert(newname.to_os_string(), ino);

        // Update the inode's parent and name
        if let Some(inode_value) = data.inodes.get_mut(&ino) {
            inode_value.parent = newparent;
            inode_value.name = newname.to_os_string();
        }
    }
}


pub struct PathResolver {
    resolver: ComponentsResolver,
}

impl GetConverter for PathBuf {
    type Resolver = PathResolver;
    fn get_converter() -> Self::Resolver {
        PathResolver::new()
    }
}

impl FileIdResolver for PathResolver {
    type FileIdType = PathBuf;
    
    fn new() -> Self {
        PathResolver {
            resolver: ComponentsResolver::new(),
        }
    }
    
    fn resolve_id(&self, ino: u64) -> Self::FileIdType {
        self.resolver.resolve_id(ino)
            .iter().rev()
            .collect::<PathBuf>()
    }
    
    fn lookup(
        &self,
        parent: u64,
        child: &OsStr,
        id: <Self::FileIdType as FileIdType>::_Id,
        increment: bool,
    ) -> u64 {
        self.resolver.lookup(parent, child, id, increment)
    }
    
    fn add_children(
        &self,
        parent: u64,
        children: Vec<(OsString, <Self::FileIdType as FileIdType>::_Id)>,
        increment: bool,
    ) -> Vec<(OsString, u64)> {
        self.resolver.add_children(parent, children, increment)
    }
    
    fn forget(&self, ino: u64, nlookup: u64) {
        self.resolver.forget(ino, nlookup);
    }
    
    fn rename(&self, parent: u64, name: &OsStr, newparent: u64, newname: &OsStr) {
        self.resolver.rename(parent, name, newparent, newname);
    }

}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn test_new() {
        let converter = ComponentsResolver::new();

        // Check if ROOT_INODE exists in the inodes HashMap
        let data = converter.data.read().expect("Failed to acquire read lock");
        assert!(data.inodes.contains_key(&ROOT_INODE));

        // Check if next_inode is correctly initialized
        assert_eq!(converter.next_inode.load(Ordering::SeqCst), 2);

        // Check properties of the root inode
        let root_inode = data.inodes.get(&ROOT_INODE).expect("Root inode not found");
        assert_eq!(root_inode.parent, ROOT_INODE);
        assert_eq!(root_inode.name, OsString::from(""));
        assert_eq!(root_inode.nlookup.load(Ordering::SeqCst), 0);

        // Check if root inode has no children initially
        assert!(data.all_children.get(&ROOT_INODE).unwrap().is_empty());
    }

    #[test]
    fn test_resolve_id() {
        let converter = ComponentsResolver::new();

        // Map shallow and nested paths
        let shallow_ino = converter.lookup(ROOT_INODE, OsStr::new("shallow_file"), (), true);
        let nested_ino = converter.lookup(shallow_ino, OsStr::new("nested_file"), (), true);

        // Test shallow path
        let shallow_path = converter.resolve_id(shallow_ino);
        assert_eq!(shallow_path, ["shallow_file"]);

        // Test nested path
        let nested_path = converter.resolve_id(nested_ino);
        assert_eq!(nested_path, ["nested_file", "shallow_file"]);

        // Verify internal state
        let data = converter.data.read().expect("Failed to acquire read lock");

        // Check shallow inode
        let shallow_inode = data
            .inodes
            .get(&shallow_ino)
            .expect("Shallow inode not found");
        assert_eq!(shallow_inode.parent, ROOT_INODE);
        assert_eq!(shallow_inode.name, OsString::from("shallow_file"));
        assert_eq!(shallow_inode.nlookup.load(Ordering::SeqCst), 1);

        // Check nested inode
        let nested_inode = data
            .inodes
            .get(&nested_ino)
            .expect("Nested inode not found");
        assert_eq!(nested_inode.parent, shallow_ino);
        assert_eq!(nested_inode.name, OsString::from("nested_file"));
        assert_eq!(nested_inode.nlookup.load(Ordering::SeqCst), 1);

        // Check children relationships
        assert!(data
            .all_children
            .get(&ROOT_INODE)
            .unwrap()
            .contains_key(OsStr::new("shallow_file")));
        assert!(data
            .all_children
            .get(&shallow_ino)
            .unwrap()
            .contains_key(OsStr::new("nested_file")));
    }

    #[test]
    fn test_path_resolver_resolve_id() {
        let converter = PathResolver::new();

        // Map shallow and nested paths
        let shallow_ino = converter.lookup(ROOT_INODE, OsStr::new("shallow_file"), (), true);
        let nested_ino = converter.lookup(shallow_ino, OsStr::new("nested_file"), (), true);

        // Test shallow path
        let shallow_path = converter.resolve_id(shallow_ino);
        assert_eq!(shallow_path, Path::new("shallow_file"));

        // Test nested path
        let nested_path = converter.resolve_id(nested_ino);
        assert_eq!(nested_path, Path::new("shallow_file").join("nested_file"));

        // Verify internal state
        let data = converter.resolver.data.read().expect("Failed to acquire read lock");

        // Check shallow inode
        let shallow_inode = data
            .inodes
            .get(&shallow_ino)
            .expect("Shallow inode not found");
        assert_eq!(shallow_inode.parent, ROOT_INODE);
        assert_eq!(shallow_inode.name, OsString::from("shallow_file"));
        assert_eq!(shallow_inode.nlookup.load(Ordering::SeqCst), 1);

        // Check nested inode
        let nested_inode = data
            .inodes
            .get(&nested_ino)
            .expect("Nested inode not found");
        assert_eq!(nested_inode.parent, shallow_ino);
        assert_eq!(nested_inode.name, OsString::from("nested_file"));
        assert_eq!(nested_inode.nlookup.load(Ordering::SeqCst), 1);

        // Check children relationships
        assert!(data
            .all_children
            .get(&ROOT_INODE)
            .unwrap()
            .contains_key(OsStr::new("shallow_file")));
        assert!(data
            .all_children
            .get(&shallow_ino)
            .unwrap()
            .contains_key(OsStr::new("nested_file")));
    }

    #[test]
    fn test_get_or_create_inode() {
        let converter = ComponentsResolver::new();

        // Map shallow and nested paths
        let shallow_ino = converter.lookup(ROOT_INODE, OsStr::new("dir"), (), true);
        let nested_ino = converter.lookup(shallow_ino, OsStr::new("file"), (), true);

        // Get data
        let data = converter.data.read().expect("Failed to acquire read lock");

        // Verify shallow path
        assert_eq!(shallow_ino, 2);
        assert!(data.inodes.contains_key(&shallow_ino));
        let shallow_inode = data.inodes.get(&shallow_ino).unwrap();
        assert_eq!(shallow_inode.parent, ROOT_INODE);
        assert_eq!(shallow_inode.name, "dir");

        // Verify nested path
        assert!(data.inodes.contains_key(&nested_ino));
        let nested_inode = data.inodes.get(&nested_ino).unwrap();
        assert_eq!(nested_inode.parent, shallow_ino);
        assert_eq!(nested_inode.name, "file");

        // Verify children relationships
        assert!(data
            .all_children
            .get(&ROOT_INODE)
            .unwrap()
            .contains_key(OsStr::new("dir")));
        assert!(data
            .all_children
            .get(&shallow_ino)
            .unwrap()
            .contains_key(OsStr::new("file")));

        // Verify lookup counts
        assert_eq!(shallow_inode.nlookup.load(Ordering::SeqCst), 1);
        assert_eq!(nested_inode.nlookup.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_rename() {
        let converter = ComponentsResolver::new();

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

        // Verify renamed shallow path
        let renamed_shallow_path = converter.resolve_id(shallow_ino);
        assert_eq!(renamed_shallow_path, ["new_dir"]);

        // Verify nested path after rename
        let renamed_nested_path = converter.resolve_id(nested_ino);
        assert_eq!(renamed_nested_path, ["file", "new_dir"]);

        // Verify internal state
        {
            let data = converter.data.read().expect("Failed to acquire read lock");

            // Check that the old name is removed from ROOT_INODE's children
            assert!(!data
                .all_children
                .get(&ROOT_INODE)
                .unwrap()
                .contains_key(OsStr::new("dir")));

            // Check that the new name is added to ROOT_INODE's children
            assert!(data
                .all_children
                .get(&ROOT_INODE)
                .unwrap()
                .contains_key(OsStr::new("new_dir")));

            // Verify the renamed inode's properties
            let renamed_inode = data
                .inodes
                .get(&shallow_ino)
                .expect("Renamed inode not found");
            assert_eq!(renamed_inode.parent, ROOT_INODE);
            assert_eq!(renamed_inode.name, OsString::from("new_dir"));

            // Verify that the nested inode's parent hasn't changed
            let nested_inode = data
                .inodes
                .get(&nested_ino)
                .expect("Nested inode not found");
            assert_eq!(nested_inode.parent, shallow_ino);
        }
    }

    #[test]
    fn test_forget() {
        let converter = ComponentsResolver::new();

        let shallow_ino = converter.lookup(ROOT_INODE, OsStr::new("dir"), (), true);
        let nested_ino = converter.lookup(shallow_ino, OsStr::new("file"), (), true);

        // Remove nested path
        converter.forget(nested_ino, 1);

        // Verify nested path is removed
        {
            let data = converter.data.read().expect("Failed to acquire read lock");
            assert!(!data.inodes.contains_key(&nested_ino));
            let shallow_children = data
                .all_children
                .get(&shallow_ino)
                .expect("Shallow inode not found");
            assert!(!shallow_children.contains_key(OsStr::new("file")));
        }

        // Remove shallow path
        converter.forget(shallow_ino, 1);

        // Verify shallow path is removed
        {
            let data = converter.data.read().expect("Failed to acquire read lock");
            assert!(!data.inodes.contains_key(&shallow_ino));
            let root_children = data
                .all_children
                .get(&ROOT_INODE)
                .expect("Root inode not found");
            assert!(!root_children.contains_key(OsStr::new("dir")));
        }
    }
}
