use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    fmt::Debug,
    hash::Hash,
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
};

use std::sync::{RwLock, atomic::AtomicU64};

use crate::{inode_mapper, types::*};
use crate::{inode_mapper::InodeMapper, inode_multi_mapper::*};

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

impl<BackingId> InodeResolvable for HybridId<BackingId>
where
    BackingId: Clone + Eq + Hash + Send + Sync + Debug + 'static,
{
    type Resolver = HybridResolver<BackingId>;

    fn create_resolver() -> Self::Resolver {
        HybridResolver::new()
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
    fn lookup_root(&self, id: <Self::ResolvedType as FileIdType>::_Id) -> ();
    fn add_children(
        &self,
        parent: u64,
        children: Vec<(OsString, <Self::ResolvedType as FileIdType>::_Id)>,
        increment: bool,
    ) -> Vec<(OsString, u64)>;
    fn forget(&self, ino: u64, nlookup: u64);
    fn prune(&self, keep: &HashSet<Self::ResolvedType>);
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

    fn lookup_root(&self, _id: <Self::ResolvedType as FileIdType>::_Id) -> () {}

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

    fn prune(&self, _keep: &HashSet<Self::ResolvedType>) {}

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
        // This scenario happens if the child node does not exist or the backing ID does not match
        u64::from(
            self.mapper
                .write()
                .expect("Failed to acquire write lock")
                .insert_child(&parent, child.to_os_string(), |_| {
                    // If the child node already exists, use the existing reference count
                    AtomicU64::new(if increment { 1 } else { 0 })
                })
                .expect("Failed to insert child"),
        )
    }

    fn lookup_root(&self, _id: <Self::ResolvedType as FileIdType>::_Id) -> () {}

    fn add_children(
        &self,
        parent: u64,
        children: Vec<(OsString, ())>,
        increment: bool,
    ) -> Vec<(OsString, u64)> {
        let value_creator = |value_creator: inode_mapper::ValueCreatorParams<AtomicU64>| {
            if let Some(nlookup) = value_creator.existing_data {
                let count = nlookup.load(Ordering::Relaxed);
                AtomicU64::new(if increment { count + 1 } else { count })
            } else {
                AtomicU64::new(if increment { 1 } else { 0 })
            }
        };
        let children_with_creator: Vec<_> = children
            .iter()
            .map(|(name, _)| (name.clone(), value_creator))
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

    fn prune(&self, keep: &HashSet<Self::ResolvedType>) {
        self.mapper
            .write()
            .expect("Failed to acquire write lock")
            .prune(keep);
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

    fn lookup_root(&self, _id: <Self::ResolvedType as FileIdType>::_Id) -> () {}

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

    fn prune(&self, keep: &HashSet<Self::ResolvedType>) {
        let resolver_keep: HashSet<Vec<OsString>> = keep
            .iter()
            .map(|path| path.iter().map(|s| s.to_os_string()).collect())
            .collect();
        self.resolver.prune(&resolver_keep);
    }

    fn rename(&self, parent: u64, name: &OsStr, newparent: u64, newname: &OsStr) {
        self.resolver.rename(parent, name, newparent, newname);
    }
}

pub struct HybridResolver<BackingId>
where
    BackingId: Clone + Eq + Hash,
{
    mapper: Arc<RwLock<InodeMultiMapper<AtomicU64, BackingId>>>,
}

impl<BackingId> FileIdResolver for HybridResolver<BackingId>
where
    BackingId: Clone + Eq + Hash + Send + Sync + std::fmt::Debug + 'static,
{
    type ResolvedType = HybridId<BackingId>;

    fn new() -> Self {
        let instance = Arc::new(RwLock::new(InodeMultiMapper::new(AtomicU64::new(0))));
        HybridResolver { mapper: instance }
    }

    fn resolve_id(&self, ino: u64) -> Self::ResolvedType {
        HybridId::new(Inode::from(ino))
    }

    fn lookup(
        &self,
        parent: u64,
        child: &OsStr,
        id: <Self::ResolvedType as FileIdType>::_Id,
        increment: bool,
    ) -> u64 {
        let parent = Inode::from(parent);
        {
            // Optimistically assume the child exists
            if let Some(lookup_result) = self
                .mapper
                .read()
                .expect("cannot acquire read lock")
                .lookup(&parent, child)
            {
                // Backing ID must match to use the hot path
                if lookup_result.backing_id.cloned() == id {
                    if increment {
                        lookup_result.data.fetch_add(1, Ordering::SeqCst);
                    }
                    return u64::from(lookup_result.inode.clone());
                }
            }
        }
        // This scenario happens if the child node does not exist or the backing ID does not match
        u64::from(
            self.mapper
                .write()
                .expect("Failed to acquire write lock")
                .insert_child(&parent, child.to_os_string(), id, |params| {
                    // If the child node already exists, use the existing reference count
                    let mut new_value = params
                        .existing_data
                        .map(|d| d.load(Ordering::SeqCst))
                        .unwrap_or(0);
                    if increment {
                        new_value += 1;
                    }
                    AtomicU64::new(new_value)
                })
                .expect("Failed to insert child"),
        )
    }

    fn lookup_root(&self, id: <Self::ResolvedType as FileIdType>::_Id) -> () {
        self.mapper
            .write()
            .expect("Failed to acquire write lock")
            .set_root_inode_backing_id(id);
    }

    fn add_children(
        &self,
        parent: u64,
        children: Vec<(OsString, <Self::ResolvedType as FileIdType>::_Id)>,
        increment: bool,
    ) -> Vec<(OsString, u64)> {
        let value_creator = |value_creator: ValueCreatorParams<AtomicU64>| {
            if let Some(nlookup) = value_creator.existing_data {
                let count = nlookup.load(Ordering::Relaxed);
                AtomicU64::new(if increment { count + 1 } else { count })
            } else {
                AtomicU64::new(if increment { 1 } else { 0 })
            }
        };
        let children_with_creator: Vec<_> = children
            .iter()
            .map(|(name, id)| (name.clone(), id.clone(), value_creator))
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
        self.mapper
            .write()
            .expect("Failed to acquire write lock")
            .remove(&inode)
            .unwrap();
    }

    fn prune(&self, _keep: &HashSet<Self::ResolvedType>) {
        let resolver_keep: HashSet<Inode> = _keep.iter().map(|id| id.inode().clone()).collect();
        self.mapper
            .write()
            .expect("Failed to acquire write lock")
            .prune(&resolver_keep);
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridIdNotFound {}

/// Specialized methods for hybrid resolver to deal with file paths and backing IDs.
impl<BackingId> HybridResolver<BackingId>
where
    BackingId: Clone + Eq + Hash + Send + Sync + std::fmt::Debug + 'static,
{
    /// Retrieves the first path to the hybrid ID's inode. This is a convenience method
    /// for users who do not need a more exhaustive list of paths that might occupy an inode.
    ///
    /// # Returns
    /// - `Ok(Some(path))` if the inode is found and the path is resolved.
    /// - `Ok(None)` if the inode is found, but it belongs to an orphaned tree.
    /// - `Err(HybridIdNotFound)` if the inode is not found at all. This usually happens
    /// when you hold an inode past its lifetime, which ends at the last forget() call that
    /// sets its lookup count to 0.
    ///
    /// # Notes
    /// - Due to the nature of an inode being able to have multiple links, there can be
    /// multiple combinations of path components that resolve to the same inode. This method
    /// only returns the first combination of path components that resolves to the inode.
    pub fn first_path(
        &self,
        id: &HybridId<BackingId>,
    ) -> Result<Option<PathBuf>, HybridIdNotFound> {
        let mapper = self
            .mapper
            .read()
            .expect("failed to acquire read lock on mapper");
        let path = mapper
            .resolve(id.inode())
            .map_err(|_| HybridIdNotFound {})?
            .map(|components| {
                components
                    .into_iter()
                    .map(|component| component.name.as_ref())
                    .rev()
                    .collect::<PathBuf>()
            });
        Ok(path)
    }

    /// Retrieves all paths to the hybrid ID's inode, up to a given limit.
    ///
    /// Inodes that are not found will result in an empty vector.
    pub fn all_paths(
        &self,
        id: &HybridId<BackingId>,
        limit: Option<usize>,
    ) -> Result<Vec<PathBuf>, HybridIdNotFound> {
        let mapper = self
            .mapper
            .read()
            .expect("failed to acquire read lock on mapper");
        let resolved = mapper
            .resolve_all(id.inode(), limit)
            .map_err(|_| HybridIdNotFound {})?;
        let paths = resolved
            .iter()
            .map(|components| {
                components
                    .iter()
                    .rev()
                    .map(|component| component.name.as_ref())
                    .collect::<PathBuf>()
            })
            .collect();
        Ok(paths)
    }

    /// Retrieves the backing ID of the inode.
    ///
    /// This is useful for comparing to the backing ID of the actual underlying
    /// file that a filesystem handler opened, which mitigates the risk of a race
    /// condition, in which case another backing path could be tried, or an error
    /// could be returned. The stable backing ID can also be used as key for the
    /// inode's data as defined by the user.
    pub fn backing_id(
        &self,
        id: &HybridId<BackingId>,
    ) -> Result<Option<BackingId>, HybridIdNotFound> {
        let mapper = self
            .mapper
            .read()
            .expect("failed to acquire read lock on mapper");
        Ok(mapper
            .get_backing_id(id.inode())
            .map_err(|_| HybridIdNotFound {})?
            .cloned())
    }

    /// Invalidate a hybrid ID's path, which is useful when a path from [`Self::all_paths`]
    /// no longer represents the hybrid ID due to upstream changes. A full path that does not
    /// start with a forward slash is expected.
    ///
    /// The method ignores paths that do not point to the inode.
    pub fn invalidate_path_of_id(
        &self,
        id: &HybridId<BackingId>,
        path: &Path,
    ) -> Result<(), HybridIdNotFound> {
        let mut mapper = self
            .mapper
            .write()
            .expect("failed to acquire write lock on mapper");
        mapper
            .invalidate_inode_path(
                id.inode(),
                &path
                    .components()
                    .map(|c| c.as_os_str().to_os_string())
                    .collect::<Vec<_>>(),
            )
            .map_err(|_| HybridIdNotFound {})?;
        Ok(())
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

        // Test prune
        let keep = HashSet::new();
        resolver.prune(&keep);

        // child_ino should be gone now because refcount was 0 (decremented by earlier forget) and we pruned it.
        // We can verify it's gone by trying to resolve it and expecting panic (as per other test) or just by knowing prune works.
        // But calling forget again is definitely wrong if it's gone.

        // If we want to test that prune actually removed it, we should check existence.
        // But since we can't easily check existence without internal access, we rely on the fact that subsequent operations might fail or the other test.
    }

    #[test]
    #[should_panic(expected = "Failed to resolve inode")]
    fn test_components_resolver_prune_panics_on_resolved_deleted() {
        let resolver = ComponentsResolver::new();
        let parent_ino = ROOT_INO;
        let child_ino = resolver.lookup(parent_ino, OsStr::new("child"), (), true);
        resolver.forget(child_ino, 1);
        resolver.prune(&HashSet::new());
        resolver.resolve_id(child_ino);
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
    fn test_hybrid_resolver() {
        let resolver = HybridResolver::<u64>::new();

        // Test lookup and resolve_id for root
        let root_ino = ROOT_INODE.into();
        let root_id = resolver.resolve_id(root_ino);
        assert_eq!(
            resolver
                .first_path(&root_id)
                .expect("root inode should be found"),
            Some(PathBuf::from(""))
        );

        // Test lookup and resolve_id for child and create nested structures
        let dir1_ino = resolver.lookup(root_ino, OsStr::new("dir1"), Some(1), true);
        let dir1_id = resolver.resolve_id(dir1_ino);
        assert_eq!(
            resolver
                .first_path(&dir1_id)
                .expect("dir1 inode should be found"),
            Some(PathBuf::from("dir1"))
        );

        let dir2_ino = resolver.lookup(dir1_ino, OsStr::new("dir2"), Some(2), true);
        let dir2_id = resolver.resolve_id(dir2_ino);
        assert_eq!(
            resolver
                .first_path(&dir2_id)
                .expect("dir2 inode should be found"),
            Some(PathBuf::from("dir1/dir2"))
        );

        // Test add_children
        let grandchildren = vec![
            (OsString::from("grandchild1"), Some(3)),
            (OsString::from("grandchild2"), Some(4)),
        ];
        let added_grandchildren = resolver.add_children(dir2_ino, grandchildren, true);
        assert_eq!(added_grandchildren.len(), 2);
        for (name, ino) in added_grandchildren.iter() {
            let child_id = resolver.resolve_id(*ino);
            assert_eq!(
                resolver
                    .first_path(&child_id)
                    .expect("child inode of dir1/dir2 should be found"),
                Some(PathBuf::from("dir1/dir2").join(name))
            );
        }

        // Test forget
        resolver.forget(added_grandchildren[0].1, 1);

        // Test rename within the same directory
        resolver.rename(
            dir2_ino,
            OsStr::new("grandchild2"),
            dir2_ino,
            OsStr::new("grandchild2_renamed"),
        );
        let renamed_grandchild_id = resolver.resolve_id(added_grandchildren[1].1);
        assert_eq!(
            resolver
                .first_path(&renamed_grandchild_id)
                .expect("renamed grandchild inode (dir1/dir2/grandchild2_renamed) should be found"),
            Some(PathBuf::from("dir1/dir2/grandchild2_renamed"))
        );

        // Test rename to a different directory
        let dir3_ino = resolver.lookup(root_ino, OsStr::new("dir3"), Some(5), true);
        resolver.rename(
            dir2_ino,
            OsStr::new("grandchild2_renamed"),
            dir3_ino,
            OsStr::new("grandchild2_renamed"),
        );
        let renamed_grandchild_id = resolver.resolve_id(added_grandchildren[1].1);
        assert_eq!(
            resolver
                .first_path(&renamed_grandchild_id)
                .expect("renamed grandchild inode (dir3/grandchild2_renamed) should exist"),
            Some(PathBuf::from("dir3/grandchild2_renamed"))
        );

        // Test lookup for non-existent file
        let non_existent_ino =
            resolver.lookup(root_ino, OsStr::new("non_existent"), Some(6), false);
        assert_ne!(non_existent_ino, 0);
        let non_existent_path = resolver.resolve_id(non_existent_ino);
        assert_eq!(
            resolver
                .first_path(&non_existent_path)
                .expect("ghost inode explicitly inserted with refcount = 0 should exist"),
            Some(PathBuf::from("non_existent"))
        );

        // Test lookup for a file with existing backing ID
        let hard_link_ino = resolver.lookup(root_ino, OsStr::new("hard_link"), Some(7), true);
        let hard_link_id = resolver.resolve_id(hard_link_ino);
        assert_eq!(
            resolver
                .first_path(&hard_link_id)
                .expect("hard link inode should be found"),
            Some(PathBuf::from("hard_link"))
        );

        let hard_link_ino_2 = resolver.lookup(dir2_ino, OsStr::new("hard_linked"), Some(7), true);
        let hard_link_id_2 = resolver.resolve_id(hard_link_ino_2);
        assert_eq!(
            hard_link_ino_2, hard_link_ino,
            "hard link inodes should be the same if callers supply the same backing ID"
        );
        assert_eq!(
            hard_link_id_2, hard_link_id,
            "hard link IDs should be the same if callers supply the same backing ID"
        );

        resolver.lookup(dir1_ino, OsStr::new("hard_linked_2"), Some(7), true);
        let paths = resolver
            .all_paths(&hard_link_id_2, Some(100))
            .expect("hard_link_id_2 should wrap an existing inode");
        assert!(paths.contains(&PathBuf::from("dir1/dir2/hard_linked")));
        assert!(paths.contains(&PathBuf::from("hard_link")));
        assert!(paths.contains(&PathBuf::from("dir1/hard_linked_2")));

        // Overriding a location with a new backing ID should always create a new inode
        let overridden_hard_link_ino =
            resolver.lookup(dir2_ino, OsStr::new("hard_linked"), Some(8), true);
        assert_ne!(
            overridden_hard_link_ino, hard_link_ino,
            "overridden location's inode should change upon encountering a new ID"
        );

        // Test path resolution after overriding a location with a new backing ID
        let paths = resolver
            .all_paths(&hard_link_id_2, Some(100))
            .expect("hard_link_id_2 should wrap an existing inode");
        assert!(
            !paths.contains(&PathBuf::from("dir1/dir2/hard_linked")),
            "the path list should no longer contain the overridden location"
        );
        assert!(paths.contains(&PathBuf::from("hard_link")));
        assert!(paths.contains(&PathBuf::from("dir1/hard_linked_2")));

        // Test invalidate_path_of_id
        resolver
            .invalidate_path_of_id(&hard_link_id_2, &PathBuf::from("dir1/hard_linked_2"))
            .expect("hard_link_id_2 should wrap an existing inode");
        let paths = resolver
            .all_paths(&hard_link_id_2, Some(100))
            .expect("hard_link_id_2 should wrap an existing inode");
        assert!(
            !paths.contains(&PathBuf::from("dir1/hard_linked_2")),
            "the path list should no longer contain the invalidated location"
        );
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
