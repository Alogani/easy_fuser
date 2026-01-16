use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
    sync::atomic::Ordering,
};

use std::sync::{atomic::AtomicU64, RwLock};

use crate::inode_mapper::*;
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
            mapper: RwLock::new(InodeMapper::new(AtomicU64::new(0))),
        }
    }

    fn resolve_id(&self, ino: u64) -> Self::ResolvedType {
        self.mapper
            .read()
            .unwrap()
            .resolve(&Inode::from(ino))
            .expect("Failed to resolve inode")
            .iter()
            .map(|inode_info| (**inode_info.name).clone())
            .collect()
    }

    fn lookup(&self, parent: u64, child: &OsStr, _id: (), increment: bool) -> u64 {
        let parent = Inode::from(parent);
        {
            // Optimistically assume the child exists
            if let Some(lookup_result) = self.mapper.read().unwrap().lookup(&parent, child) {
                if increment {
                    lookup_result.data.fetch_add(1, Ordering::SeqCst);
                }
                return u64::from(lookup_result.inode.clone());
            }
        }
        u64::from(
            self.mapper
                .write()
                .expect("Failed to acquire write lock")
                .insert_child(&parent, child.to_os_string(), |_| {
                    AtomicU64::new(if increment { 1 } else { 0 })
                })
                .expect("Failed to insert child"),
        )
    }

    fn add_children(
        &self,
        parent: u64,
        children: Vec<(OsString, ())>,
        increment: bool,
    ) -> Vec<(OsString, u64)> {
        let children_with_creator: Vec<_> = children
            .iter()
            .map(|(name, _)| {
                (
                    name.clone(),
                    |value_creator: ValueCreatorParams<AtomicU64>| match value_creator.existing_data
                    {
                        Some(nlookup) => {
                            let count = nlookup.load(Ordering::Relaxed);
                            AtomicU64::new(if increment { count + 1 } else { count })
                        }
                        None => AtomicU64::new(if increment { 1 } else { 0 }),
                    },
                )
            })
            .collect();

        let parent_inode = Inode::from(parent);
        let inserted_children = self
            .mapper
            .write()
            .expect("Failed to acquire write lock")
            .insert_children(&parent_inode, children_with_creator)
            .expect("Failed to insert children");

        inserted_children
            .into_iter()
            .zip(children)
            .map(|(inode, (name, _))| (name, u64::from(inode)))
            .collect()
    }

    fn forget(&self, ino: u64, nlookup: u64) {
        let inode = Inode::from(ino);
        {
            // Optimistically assume we don't have to remove yet
            let guard = self.mapper.read().expect("Failed to acquire read lock");
            let inode_info = guard.get(&inode).expect("Failed to find inode");
            if inode_info.data.fetch_sub(nlookup, Ordering::SeqCst) > 0 {
                return;
            }
        }
        self.mapper.write().unwrap().remove(&inode).unwrap();
    }

    fn rename(&self, parent: u64, name: &OsStr, newparent: u64, newname: &OsStr) {
        let parent_inode = Inode::from(parent);
        let newparent_inode = Inode::from(newparent);
        self.mapper
            .write()
            .expect("Failed to acquire write lock")
            .rename(
                &parent_inode,
                name,
                &newparent_inode,
                newname.to_os_string(),
            )
            .expect("Failed to rename inode");
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
    use super::*;
    use std::ffi::OsStr;
    use std::path::PathBuf;

    #[test]
    fn test_components_resolver() {
        let resolver = ComponentsResolver::new();

        // Test lookup and resolve_id
        let parent_ino = ROOT_INODE.into();
        let child_ino = resolver.lookup(parent_ino, OsStr::new("child"), (), true);
        let resolved_path = resolver.resolve_id(child_ino);

        assert_eq!(resolved_path, vec![OsString::from("child")]);

        // Test add_children
        let grandchildren = vec![
            (OsString::from("grandchild1"), ()),
            (OsString::from("grandchild2"), ()),
        ];
        let added_children = resolver.add_children(child_ino, grandchildren, true);

        assert_eq!(added_children.len(), 2);

        // Test forget
        resolver.forget(child_ino, 1);

        // Test rename
        resolver.rename(
            parent_ino,
            OsStr::new("child"),
            parent_ino,
            OsStr::new("renamed_child"),
        );

        let renamed_path = resolver.resolve_id(child_ino);
        assert_eq!(renamed_path, vec![OsString::from("renamed_child")]);
    }

    #[test]
    fn test_path_resolver() {
        let resolver = PathResolver::new();

        // Test lookup and resolve_id for root
        let root_ino = ROOT_INODE.into();
        let root_path = resolver.resolve_id(root_ino);
        assert_eq!(root_path, PathBuf::from(""));

        // Create a nested structure: /dir1/dir2/file.txt
        let dir1_ino = resolver.lookup(root_ino, OsStr::new("dir1"), (), true);
        let dir2_ino = resolver.lookup(dir1_ino, OsStr::new("dir2"), (), true);
        let file_ino = resolver.lookup(dir2_ino, OsStr::new("file.txt"), (), true);

        // Test resolve_id for nested structure
        let file_path = resolver.resolve_id(file_ino);
        assert_eq!(file_path, PathBuf::from("dir1/dir2/file.txt"));

        // Test add_children
        let dir2_children = vec![
            (OsString::from("child1.txt"), ()),
            (OsString::from("child2.txt"), ()),
        ];
        let added_children = resolver.add_children(dir2_ino, dir2_children, true);
        assert_eq!(added_children.len(), 2);

        // Verify added children
        for (name, ino) in added_children {
            let child_path = resolver.resolve_id(ino);
            assert_eq!(
                child_path,
                PathBuf::from(format!("dir1/dir2/{}", name.to_str().unwrap()))
            );
        }

        // Test forget
        resolver.forget(file_ino, 1);

        // Test rename within the same directory
        resolver.rename(
            dir2_ino,
            OsStr::new("file.txt"),
            dir2_ino,
            OsStr::new("renamed_file.txt"),
        );

        let renamed_file_path = resolver.resolve_id(file_ino);
        assert_eq!(
            renamed_file_path,
            PathBuf::from("dir1/dir2/renamed_file.txt")
        );

        // Test rename to a different directory
        let dir3_ino = resolver.lookup(root_ino, OsStr::new("dir3"), (), true);
        resolver.rename(
            dir2_ino,
            OsStr::new("renamed_file.txt"),
            dir3_ino,
            OsStr::new("moved_file.txt"),
        );

        let moved_file_path = resolver.resolve_id(file_ino);
        assert_eq!(moved_file_path, PathBuf::from("dir3/moved_file.txt"));

        // Test lookup for non-existent file
        let non_existent_ino = resolver.lookup(root_ino, OsStr::new("non_existent"), (), false);
        assert_ne!(non_existent_ino, 0);
        let non_existent_path = resolver.resolve_id(non_existent_ino);
        assert_eq!(non_existent_path, PathBuf::from("non_existent"));
    }

    #[test]
    fn test_path_resolver_back_and_forth_rename() {
        let resolver = PathResolver::new();

        // Test lookup and resolve_id for root
        let root_ino = ROOT_INODE.into();
        let root_path = resolver.resolve_id(root_ino);
        assert_eq!(root_path, PathBuf::from(""));

        // Add directories
        let dir1_ino = resolver.lookup(root_ino, OsStr::new("dir1"), (), true);
        let dir2_ino = resolver.lookup(dir1_ino, OsStr::new("dir2"), (), true);
        let file_ino = resolver.lookup(root_ino, OsStr::new("file.txt"), (), true);

        // Rename file to a different directory
        resolver.rename(
            root_ino,
            OsStr::new("file.txt"),
            dir2_ino,
            OsStr::new("file.txt"),
        );
        let renamed_file_path = resolver.resolve_id(file_ino);
        assert_eq!(renamed_file_path, PathBuf::from("dir1/dir2/file.txt"));

        // Rename file back to original directory
        resolver.rename(
            dir2_ino,
            OsStr::new("file.txt"),
            root_ino,
            OsStr::new("file.txt"),
        );
        let renamed_file_path = resolver.resolve_id(file_ino);
        assert_eq!(renamed_file_path, PathBuf::from("file.txt"));
    }
}
