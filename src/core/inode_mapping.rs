use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::PathBuf,
    sync::atomic::Ordering,
};

use std::sync::{atomic::AtomicU64, RwLock};

use crate::types::*;

pub(crate) const ROOT_INO: u64 = 1;

/// Trait to allow a FileIdType to be mapped to use a converter
pub trait InodeResolvable {
    type Resolver: FileIdResolver<ResolvedType = Self>;

    fn create_resolver() -> Self::Resolver;
}

impl InodeResolvable for PathBuf {
    type Resolver = PathResolver;

    fn create_resolver() -> Self::Resolver {
        PathResolver::new()
    }
}

impl InodeResolvable for Inode {
    type Resolver = InodeResolver;

    fn create_resolver() -> Self::Resolver {
        InodeResolver::new()
    }
}

impl InodeResolvable for Vec<OsString> {
    type Resolver = ComponentsResolver;

    fn create_resolver() -> Self::Resolver {
        ComponentsResolver::new()
    }
}

/// FileIdResolver
/// FileIdResolver handles its data behind Locks if needed and should not be nested inside a Mutex
pub trait FileIdResolver: Send + Sync + 'static {
    type ResolvedType: FileIdType;

    fn new() -> Self;
    fn resolve_id(&self, ino: u64) -> Self::ResolvedType;
    fn lookup(
        &self,
        parent: u64,
        child: &OsStr,
        id: <Self::ResolvedType as FileIdType>::_Id,
        increment: bool,
    ) -> u64;
    fn add_children(
        &self,
        parent: u64,
        children: Vec<(OsString, <Self::ResolvedType as FileIdType>::_Id)>,
        increment: bool,
    ) -> Vec<(OsString, u64)>;
    fn forget(&self, ino: u64, nlookup: u64);
    fn rename(&self, parent: u64, name: &OsStr, newparent: u64, newname: &OsStr);
}

pub struct InodeResolver {}

impl FileIdResolver for InodeResolver {
    type ResolvedType = Inode;

    fn new() -> Self {
        Self {}
    }

    fn resolve_id(&self, ino: u64) -> Self::ResolvedType {
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
    mapper: RwLock<InodeMapper<AtomicU64>>,
}


impl FileIdResolver for ComponentsResolver {
    type ResolvedType = Vec<OsString>;

    fn new() -> Self {
        ComponentsResolver {
            mapper: RwLock::new(InodeMapper::new(AtomicU64::new(0)))
        }
    }

    fn resolve_id(&self, ino: u64) -> Self::ResolvedType {
        self
            .mapper
            .read()
            .unwrap()
            .resolve(&Inode::from(ino))
            .expect("Failed to resolve inode")
            .iter()
            .map(OsString::from)
            .collect()
    }

    fn lookup(&self, parent: u64, child: &OsStr, _id: (), increment: bool) -> u64 {
        let parent = Inode::from(parent);
        {
            // Optimistically assume the child exists
            if let Some(lookup_result) = self.mapper
                .read()
                .unwrap()
                .lookup(&parent, child) {
                    lookup_result.data.fetch_add(1, Ordering::SeqCst);
                    return lookup_result.inode.into();
                }
        }
        self.mapper
            .write()
            .unwrap()
            .insert_child(parent, child, ||)
            .unwrap()
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
                new_children.push((name.clone(), ino));
                new_inodes.push(ino);
                parent_children.insert(unsafe { OsStrPtr::from(&name) }, ino);
                inodes.insert(
                    ino,
                    InodeValue {
                        nlookup: AtomicU64::new(if increment { 1 } else { 0 }),
                        parent,
                        name,
                    },
                );
            }

            new_children
        } else {
            if children_len > parent_children.len() {
                parent_children.reserve(children_len - parent_children.len());
            }
            let mut result = Vec::with_capacity(children.len());
            let mut new_inodes = Vec::with_capacity(children_len);

            for (name, _) in children {
                if let Some(&existing_ino) = parent_children.get(&OsStrPtr::unsafe_borrow(&name)) {
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
                    new_inodes.push(new_ino);
                    result.push((name.clone(), new_ino));
                    parent_children.insert(unsafe { OsStrPtr::from(&name) }, new_ino);
                    inodes.insert(
                        new_ino,
                        InodeValue {
                            nlookup: AtomicU64::new(if increment { 1 } else { 0 }),
                            parent,
                            name,
                        },
                    );
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
                    parent_children.remove(&OsStrPtr::unsafe_borrow(&name));
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
            children.get(&OsStrPtr::unsafe_borrow(&name)),
            "Rename called on non existent child {:?} {:?}",
            parent,
            name
        );

        children.remove(&OsStrPtr::unsafe_borrow(&name));

        let new_parent_children = unwrap!(
            data.all_children.get_mut(&newparent),
            "No such newparent inode {}",
            parent
        );
        new_parent_children.insert(unsafe { OsStrPtr::from(&newname) }, ino);

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

impl FileIdResolver for PathResolver {
    type ResolvedType = PathBuf;

    fn new() -> Self {
        PathResolver {
            resolver: ComponentsResolver::new(),
        }
    }

    fn resolve_id(&self, ino: u64) -> Self::ResolvedType {
        self.resolver
            .resolve_id(ino)
            .iter()
            .rev()
            .collect::<PathBuf>()
    }

    fn lookup(
        &self,
        parent: u64,
        child: &OsStr,
        id: <Self::ResolvedType as FileIdType>::_Id,
        increment: bool,
    ) -> u64 {
        self.resolver.lookup(parent, child, id, increment)
    }

    fn add_children(
        &self,
        parent: u64,
        children: Vec<(OsString, <Self::ResolvedType as FileIdType>::_Id)>,
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
        assert!(data.inodes.contains_key(&ROOT_INO));

        // Check if next_inode is correctly initialized
        assert_eq!(converter.next_inode.load(Ordering::SeqCst), 2);

        // Check properties of the root inode
        let root_inode = data.inodes.get(&ROOT_INO).expect("Root inode not found");
        assert_eq!(root_inode.parent, ROOT_INO);
        assert_eq!(root_inode.name, OsString::from(""));
        assert_eq!(root_inode.nlookup.load(Ordering::SeqCst), 0);

        // Check if root inode has no children initially
        assert!(data.all_children.get(&ROOT_INO).unwrap().is_empty());
    }

    #[test]
    fn test_resolve_id() {
        let converter = ComponentsResolver::new();

        // Map shallow and nested paths
        let shallow_ino = converter.lookup(ROOT_INO, OsStr::new("shallow_file"), (), true);
        let nested_ino = converter.lookup(shallow_ino, OsStr::new("nested_file"), (), true);

        // Test root path
        let root_path = converter.resolve_id(ROOT_INO);
        assert!(root_path.is_empty());

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
        assert_eq!(shallow_inode.parent, ROOT_INO);
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
            .get(&ROOT_INO)
            .unwrap()
            .contains_key(&OsStrPtr::unsafe_borrow(OsStr::new("shallow_file"))));
        assert!(data
            .all_children
            .get(&shallow_ino)
            .unwrap()
            .contains_key(&OsStrPtr::unsafe_borrow(OsStr::new("nested_file"))));
    }

    #[test]
    fn test_path_resolver_resolve_id() {
        let converter = PathResolver::new();

        // Map shallow and nested paths
        let shallow_ino = converter.lookup(ROOT_INO, OsStr::new("shallow_file"), (), true);
        let nested_ino = converter.lookup(shallow_ino, OsStr::new("nested_file"), (), true);

        // Test root path
        let root_path = converter.resolve_id(ROOT_INO);
        assert_eq!(root_path, Path::new(""));

        // Test shallow path
        let shallow_path = converter.resolve_id(shallow_ino);
        assert_eq!(shallow_path, Path::new("shallow_file"));

        // Test nested path
        let nested_path = converter.resolve_id(nested_ino);
        assert_eq!(nested_path, Path::new("shallow_file").join("nested_file"));

        // Verify internal state
        let data = converter
            .resolver
            .data
            .read()
            .expect("Failed to acquire read lock");

        // Check shallow inode
        let shallow_inode = data
            .inodes
            .get(&shallow_ino)
            .expect("Shallow inode not found");
        assert_eq!(shallow_inode.parent, ROOT_INO);
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
            .get(&ROOT_INO)
            .unwrap()
            .contains_key(&OsStrPtr::unsafe_borrow(OsStr::new("shallow_file"))));
        assert!(data
            .all_children
            .get(&shallow_ino)
            .unwrap()
            .contains_key(&OsStrPtr::unsafe_borrow(OsStr::new("nested_file"))));
    }

    #[test]
    fn test_get_or_create_inode() {
        let converter = ComponentsResolver::new();

        // Map shallow and nested paths
        let shallow_ino = converter.lookup(ROOT_INO, OsStr::new("dir"), (), true);
        let nested_ino = converter.lookup(shallow_ino, OsStr::new("file"), (), true);

        // Get data
        let data = converter.data.read().expect("Failed to acquire read lock");

        // Verify shallow path
        assert_eq!(shallow_ino, 2);
        assert!(data.inodes.contains_key(&shallow_ino));
        let shallow_inode = data.inodes.get(&shallow_ino).unwrap();
        assert_eq!(shallow_inode.parent, ROOT_INO);
        assert_eq!(shallow_inode.name, "dir");

        // Verify nested path
        assert!(data.inodes.contains_key(&nested_ino));
        let nested_inode = data.inodes.get(&nested_ino).unwrap();
        assert_eq!(nested_inode.parent, shallow_ino);
        assert_eq!(nested_inode.name, "file");

        // Verify children relationships
        assert!(data
            .all_children
            .get(&ROOT_INO)
            .unwrap()
            .contains_key(&OsStrPtr::unsafe_borrow(OsStr::new("dir"))));
        assert!(data
            .all_children
            .get(&shallow_ino)
            .unwrap()
            .contains_key(&OsStrPtr::unsafe_borrow(OsStr::new("file"))));

        // Verify lookup counts
        assert_eq!(shallow_inode.nlookup.load(Ordering::SeqCst), 1);
        assert_eq!(nested_inode.nlookup.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_rename() {
        let converter = ComponentsResolver::new();

        // Map shallow and nested paths
        let shallow_ino = converter.lookup(ROOT_INO, OsStr::new("dir"), (), true);
        let nested_ino = converter.lookup(shallow_ino, OsStr::new("file"), (), true);

        // Rename shallow path
        converter.rename(ROOT_INO, OsStr::new("dir"), ROOT_INO, OsStr::new("new_dir"));

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
                .get(&ROOT_INO)
                .unwrap()
                .contains_key(&OsStrPtr::unsafe_borrow(OsStr::new("dir"))));

            // Check that the new name is added to ROOT_INODE's children
            assert!(data
                .all_children
                .get(&ROOT_INO)
                .unwrap()
                .contains_key(&OsStrPtr::unsafe_borrow(OsStr::new("new_dir"))));

            // Verify the renamed inode's properties
            let renamed_inode = data
                .inodes
                .get(&shallow_ino)
                .expect("Renamed inode not found");
            assert_eq!(renamed_inode.parent, ROOT_INO);
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

        let shallow_ino = converter.lookup(ROOT_INO, OsStr::new("dir"), (), true);
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
            assert!(!shallow_children.contains_key(&OsStrPtr::unsafe_borrow(&OsStr::new("file"))));
        }

        // Remove shallow path
        converter.forget(shallow_ino, 1);

        // Verify shallow path is removed
        {
            let data = converter.data.read().expect("Failed to acquire read lock");
            assert!(!data.inodes.contains_key(&shallow_ino));
            let root_children = data
                .all_children
                .get(&ROOT_INO)
                .expect("Root inode not found");
            assert!(!root_children.contains_key(&OsStrPtr::unsafe_borrow(OsStr::new("dir"))));
        }
    }
}
