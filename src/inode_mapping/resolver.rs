use std::{
    collections::{HashSet, VecDeque},
    ffi::{OsStr, OsString},
    ops::Deref,
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
};

use std::sync::{
    RwLock,
    atomic::{AtomicU64, AtomicUsize},
};

use crate::{
    inode_mapping::{InodeMapper, ValueCreatorParams},
    types::*,
};

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

impl InodeResolvable for MappedInode {
    type Resolver = MappedResolver;

    fn create_resolver() -> Self::Resolver {
        MappedResolver::new()
    }
}

/// FileIdResolver
/// FileIdResolver handles its data behind Locks if needed and should not be nested inside a Mutex
pub trait FileIdResolver: Send + Sync + 'static {
    type ResolvedType: FileIdType;

    fn new() -> Self;
    fn resolve_id(&self, ino: Inode) -> Self::ResolvedType;
    fn lookup(
        &self,
        parent: Inode,
        child: &OsStr,
        id: <Self::ResolvedType as FileIdType>::_Id,
        increment: bool,
    ) -> Inode;
    fn add_children(
        &self,
        parent: Inode,
        children: Vec<(OsString, <Self::ResolvedType as FileIdType>::_Id)>,
        increment: bool,
    ) -> Vec<(OsString, Inode)>;
    fn forget(&self, ino: Inode, nlookup: u64);
    fn begin_request(&self) -> Option<RequestGuard> {
        None
    }
    fn prune(&self, keep: &HashSet<Self::ResolvedType>);
    fn rename(&self, parent: Inode, name: &OsStr, newparent: Inode, newname: &OsStr);
    fn exchange(&self, _parent: Inode, _name: &OsStr, _newparent: Inode, _newname: &OsStr) {}
    /// Returns true when the new entry has been associated with `ino`.
    fn link(&self, _ino: Inode, _parent: Inode, _name: &OsStr) -> bool {
        false
    }
    fn unlink(&self, _parent: Inode, _name: &OsStr) {}
}

pub(crate) struct RequestResolver<R: FileIdResolver> {
    resolver: Arc<R>,
    _request: Option<RequestGuard>,
}

impl<R: FileIdResolver> RequestResolver<R> {
    pub(crate) fn new(resolver: Arc<R>) -> Self {
        let request = resolver.begin_request();
        Self {
            resolver,
            _request: request,
        }
    }
}

impl<R: FileIdResolver> Deref for RequestResolver<R> {
    type Target = R;

    fn deref(&self) -> &R {
        &self.resolver
    }
}

pub struct RequestGuard {
    tracker: Arc<RequestTracker>,
    active: Arc<AtomicUsize>,
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        if self.active.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.tracker.cleanup();
        }
    }
}

#[derive(Default)]
struct RequestEpoch {
    active: Arc<AtomicUsize>,
    retired: Vec<Inode>,
}

struct RequestState {
    epochs: VecDeque<RequestEpoch>,
}

impl Default for RequestState {
    fn default() -> Self {
        Self {
            epochs: VecDeque::from([RequestEpoch::default()]),
        }
    }
}

struct RequestTracker {
    state: RwLock<RequestState>,
    mapper: Arc<RwLock<InodeMapper<AtomicU64>>>,
}

impl RequestTracker {
    fn begin(self: &Arc<Self>) -> RequestGuard {
        let state = self.state.read().unwrap();
        let active = &state.epochs.back().expect("current request epoch").active;
        active.fetch_add(1, Ordering::Relaxed);
        RequestGuard {
            tracker: self.clone(),
            active: active.clone(),
        }
    }

    fn retire(&self, ino: Inode) {
        let ready = {
            let mut state = self.state.write().unwrap();
            state.epochs.back_mut().unwrap().retired.push(ino);
            // Requests registered after this point enter a new epoch and do
            // not hold up cleanup of requests already in flight.
            state.epochs.push_back(RequestEpoch::default());
            Self::drain_ready(&mut state)
        };
        self.evict(ready);
    }

    fn cleanup(&self) {
        let ready = {
            let mut state = self.state.write().unwrap();
            Self::drain_ready(&mut state)
        };
        self.evict(ready);
    }

    fn drain_ready(state: &mut RequestState) -> Vec<Inode> {
        let mut ready = Vec::new();
        while state.epochs.len() > 1
            && state.epochs.front().unwrap().active.load(Ordering::Acquire) == 0
        {
            ready.extend(state.epochs.pop_front().unwrap().retired);
        }
        ready
    }

    fn evict(&self, inodes: Vec<Inode>) {
        if inodes.is_empty() {
            return;
        }
        let mut mapper = self.mapper.write().expect("Failed to acquire write lock");
        for ino in inodes {
            ComponentsResolver::evict_unreferenced(&mut mapper, ino);
        }
    }
}

pub struct InodeResolver {}

impl FileIdResolver for InodeResolver {
    type ResolvedType = Inode;

    fn new() -> Self {
        Self {}
    }

    fn resolve_id(&self, ino: Inode) -> Self::ResolvedType {
        ino
    }

    fn lookup(&self, _parent: Inode, _child: &OsStr, id: Inode, _increment: bool) -> Inode {
        id
    }

    fn add_children(
        &self,
        _parent: Inode,
        children: Vec<(OsString, Inode)>,
        _increment: bool,
    ) -> Vec<(OsString, Inode)> {
        children
    }

    fn forget(&self, _ino: Inode, _nlookup: u64) {}

    fn prune(&self, _keep: &HashSet<Self::ResolvedType>) {}

    fn rename(&self, _parent: Inode, _name: &OsStr, _newparent: Inode, _newname: &OsStr) {}

    fn link(&self, _ino: Inode, _parent: Inode, _name: &OsStr) -> bool {
        true
    }
}

pub struct ComponentsResolver {
    mapper: Arc<RwLock<InodeMapper<AtomicU64>>>,
    requests: Arc<RequestTracker>,
}

impl FileIdResolver for ComponentsResolver {
    type ResolvedType = Vec<OsString>;

    fn new() -> Self {
        let mapper = Arc::new(RwLock::new(InodeMapper::new(AtomicU64::new(0))));
        ComponentsResolver {
            requests: Arc::new(RequestTracker {
                state: RwLock::new(RequestState::default()),
                mapper: mapper.clone(),
            }),
            mapper,
        }
    }

    fn begin_request(&self) -> Option<RequestGuard> {
        Some(self.requests.begin())
    }

    fn resolve_id(&self, ino: Inode) -> Self::ResolvedType {
        self.mapper
            .read()
            .unwrap()
            .resolve_first(&ino)
            .expect("Failed to resolve inode")
    }

    fn lookup(&self, parent: Inode, child: &OsStr, _id: (), increment: bool) -> Inode {
        {
            // Optimistically assume the child exists
            if let Some(lookup_result) = self.mapper.read().unwrap().lookup(&parent, child) {
                if increment {
                    lookup_result.data.fetch_add(1, Ordering::SeqCst);
                }
                return lookup_result.inode.clone();
            }
        }
        // Another request may have inserted the child while the read lock was released.
        let mut mapper = self.mapper.write().expect("Failed to acquire write lock");
        if let Some(lookup_result) = mapper.lookup(&parent, child) {
            if increment {
                lookup_result.data.fetch_add(1, Ordering::SeqCst);
            }
            return *lookup_result.inode;
        }
        mapper
            .insert_child(&parent, child.to_os_string(), |_| {
                AtomicU64::new(if increment { 1 } else { 0 })
            })
            .expect("Failed to insert child")
    }

    fn add_children(
        &self,
        parent: Inode,
        children: Vec<(OsString, ())>,
        increment: bool,
    ) -> Vec<(OsString, Inode)> {
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
            .map(|(name, _)| (name.clone(), value_creator))
            .collect();
        let inserted_children = self
            .mapper
            .write()
            .expect("Failed to acquire write lock")
            .insert_children(&parent, children_with_creator)
            .expect("Failed to insert children");
        inserted_children
            .into_iter()
            .zip(children)
            .map(|(inode, (name, _))| (name, inode))
            .collect()
    }

    fn forget(&self, ino: Inode, nlookup: u64) {
        if ino == ROOT_INODE {
            return;
        }
        let mapper = self.mapper.write().expect("Failed to acquire write lock");
        let Some(info) = mapper.get(&ino) else { return };
        let previous = info
            .data
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                Some(count.saturating_sub(nlookup))
            })
            .expect("lookup count update");
        if previous <= nlookup {
            drop(mapper);
            self.requests.retire(ino);
        }
    }

    fn prune(&self, keep: &HashSet<Self::ResolvedType>) {
        self.mapper
            .write()
            .expect("Failed to acquire write lock")
            .prune(keep);
    }

    fn rename(&self, parent: Inode, name: &OsStr, newparent: Inode, newname: &OsStr) {
        self.mapper
            .write()
            .expect("Failed to acquire write lock")
            .rename(&parent, name, &newparent, newname.to_os_string())
            .expect("Failed to rename inode");
    }
}

impl ComponentsResolver {
    // A forgotten parent is needed as long as a child still has FUSE references.
    // Remove zero-count descendants first, then reconsider their parents.
    fn evict_unreferenced(mapper: &mut InodeMapper<AtomicU64>, ino: Inode) {
        if ino == ROOT_INODE {
            return;
        }
        let children: Vec<_> = mapper
            .get_children(&ino)
            .into_iter()
            .map(|(_, child)| *child)
            .collect();
        for child in children {
            if child != ino {
                Self::evict_unreferenced(mapper, child);
            }
        }
        let Some(info) = mapper.get(&ino) else { return };
        if info.data.load(Ordering::SeqCst) != 0 || !mapper.get_children(&ino).is_empty() {
            return;
        }
        let parents: Vec<_> = info.links.iter().map(|(parent, _)| **parent).collect();
        mapper.remove(&ino);
        for parent in parents {
            Self::evict_unreferenced(mapper, parent);
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

    fn resolve_id(&self, ino: Inode) -> Self::ResolvedType {
        self.resolver
            .resolve_id(ino)
            .iter()
            .rev()
            .collect::<PathBuf>()
    }

    fn lookup(
        &self,
        parent: Inode,
        child: &OsStr,
        id: <Self::ResolvedType as FileIdType>::_Id,
        increment: bool,
    ) -> Inode {
        self.resolver.lookup(parent, child, id, increment)
    }

    fn add_children(
        &self,
        parent: Inode,
        children: Vec<(OsString, <Self::ResolvedType as FileIdType>::_Id)>,
        increment: bool,
    ) -> Vec<(OsString, Inode)> {
        self.resolver.add_children(parent, children, increment)
    }

    fn forget(&self, ino: Inode, nlookup: u64) {
        self.resolver.forget(ino, nlookup);
    }

    fn begin_request(&self) -> Option<RequestGuard> {
        self.resolver.begin_request()
    }

    fn prune(&self, keep: &HashSet<Self::ResolvedType>) {
        let resolver_keep: HashSet<Vec<OsString>> = keep
            .iter()
            .map(|path| path.iter().map(|s| s.to_os_string()).collect())
            .collect();
        self.resolver.prune(&resolver_keep);
    }

    fn rename(&self, parent: Inode, name: &OsStr, newparent: Inode, newname: &OsStr) {
        self.resolver.rename(parent, name, newparent, newname);
    }

    fn exchange(&self, parent: Inode, name: &OsStr, newparent: Inode, newname: &OsStr) {
        self.resolver
            .mapper
            .write()
            .expect("Failed to acquire write lock")
            .exchange(&parent, name, &newparent, newname)
            .expect("Failed to exchange entries");
    }
}

pub struct MappedResolver {
    resolver: ComponentsResolver,
}

impl FileIdResolver for MappedResolver {
    type ResolvedType = MappedInode;

    fn new() -> Self {
        Self {
            resolver: ComponentsResolver::new(),
        }
    }

    fn resolve_id(&self, ino: Inode) -> MappedInode {
        MappedInode::new(ino, self.resolver.mapper.clone())
    }

    fn lookup(&self, parent: Inode, child: &OsStr, id: (), increment: bool) -> Inode {
        self.resolver.lookup(parent, child, id, increment)
    }

    fn add_children(
        &self,
        parent: Inode,
        children: Vec<(OsString, ())>,
        increment: bool,
    ) -> Vec<(OsString, Inode)> {
        self.resolver.add_children(parent, children, increment)
    }

    fn forget(&self, ino: Inode, nlookup: u64) {
        if ino == ROOT_INODE {
            return;
        }
        let mut mapper = self
            .resolver
            .mapper
            .write()
            .expect("Failed to acquire write lock");
        let Some(info) = mapper.get(&ino) else { return };
        let previous = info
            .data
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                Some(count.saturating_sub(nlookup))
            })
            .expect("lookup count update");
        if previous <= nlookup && !mapper.has_links(&ino) {
            mapper.remove(&ino);
        }
    }

    fn prune(&self, _keep: &HashSet<MappedInode>) {
        // A forgotten parent may still be needed to resolve a linked child's
        // path. Names are removed by unlink/rmdir/rename instead.
    }

    fn rename(&self, parent: Inode, name: &OsStr, newparent: Inode, newname: &OsStr) {
        self.resolver.rename(parent, name, newparent, newname);
    }

    fn exchange(&self, parent: Inode, name: &OsStr, newparent: Inode, newname: &OsStr) {
        self.resolver
            .mapper
            .write()
            .expect("Failed to acquire write lock")
            .exchange(&parent, name, &newparent, newname)
            .expect("Failed to exchange entries");
    }

    fn link(&self, ino: Inode, parent: Inode, name: &OsStr) -> bool {
        let mut mapper = self
            .resolver
            .mapper
            .write()
            .expect("Failed to acquire write lock");
        if mapper.lookup(&parent, name).is_some() {
            // A successful backing link means this cached entry is stale.
            mapper.unlink(&parent, name);
        }
        mapper
            .link(&ino, &parent, name.to_os_string())
            .expect("Failed to register hard link");
        mapper
            .get(&ino)
            .unwrap()
            .data
            .fetch_add(1, Ordering::SeqCst);
        true
    }

    fn unlink(&self, parent: Inode, name: &OsStr) {
        let mut mapper = self
            .resolver
            .mapper
            .write()
            .expect("Failed to acquire write lock");
        if let Some(ino) = mapper.unlink(&parent, name) {
            if !mapper.has_links(&ino)
                && mapper
                    .get(&ino)
                    .is_some_and(|info| info.data.load(Ordering::SeqCst) == 0)
            {
                mapper.remove(&ino);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::path::PathBuf;
    use std::sync::Barrier;
    use std::thread;

    #[test]
    fn concurrent_lookup_counts_every_reference() {
        let resolver = Arc::new(PathResolver::new());
        let workers = 16;
        let start = Arc::new(Barrier::new(workers));
        let threads: Vec<_> = (0..workers)
            .map(|_| {
                let resolver = resolver.clone();
                let start = start.clone();
                thread::spawn(move || {
                    start.wait();
                    resolver.lookup(ROOT_INODE, OsStr::new("shared"), (), true)
                })
            })
            .collect();
        let inodes: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect();
        assert!(inodes.iter().all(|inode| *inode == inodes[0]));
        let mapper = resolver.resolver.mapper.read().unwrap();
        assert_eq!(
            mapper.get(&inodes[0]).unwrap().data.load(Ordering::SeqCst),
            workers as u64
        );
    }

    #[test]
    fn final_forget_releases_path_mapping() {
        let resolver = PathResolver::new();
        for index in 0..100 {
            let name = OsString::from(format!("ephemeral-{index}"));
            let ino = resolver.lookup(ROOT_INODE, &name, (), true);
            resolver.forget(ino, 1);
            let mapper = resolver.resolver.mapper.read().unwrap();
            assert!(mapper.get(&ino).is_none());
            assert!(mapper.lookup(&ROOT_INODE, &name).is_none());
        }
    }

    #[test]
    fn partial_forget_keeps_path_mapping() {
        let resolver = PathResolver::new();
        let ino = resolver.lookup(ROOT_INODE, OsStr::new("shared"), (), true);
        assert_eq!(
            resolver.lookup(ROOT_INODE, OsStr::new("shared"), (), true),
            ino
        );
        resolver.forget(ino, 1);
        assert_eq!(resolver.resolve_id(ino), PathBuf::from("shared"));
        resolver.forget(ino, 1);
        assert!(resolver.resolver.mapper.read().unwrap().get(&ino).is_none());
    }

    #[test]
    fn queued_request_finishes_before_mapping_is_retired() {
        let resolver = Arc::new(PathResolver::new());
        let ino = resolver.lookup(ROOT_INODE, OsStr::new("queued"), (), true);
        let earlier = RequestResolver::new(resolver.clone());
        resolver.forget(ino, 1);
        assert_eq!(earlier.resolve_id(ino), PathBuf::from("queued"));
        let later = RequestResolver::new(resolver.clone());
        drop(earlier);
        assert!(resolver.resolver.mapper.read().unwrap().get(&ino).is_none());
        drop(later);
    }

    #[test]
    fn retired_generations_are_released_in_order() {
        let resolver = Arc::new(PathResolver::new());
        let first = resolver.lookup(ROOT_INODE, OsStr::new("first"), (), true);
        let second = resolver.lookup(ROOT_INODE, OsStr::new("second"), (), true);
        let early = RequestResolver::new(resolver.clone());
        resolver.forget(first, 1);
        let middle = RequestResolver::new(resolver.clone());
        resolver.forget(second, 1);
        let late = RequestResolver::new(resolver.clone());
        drop(early);
        let mapper = resolver.resolver.mapper.read().unwrap();
        assert!(mapper.get(&first).is_none());
        assert!(mapper.get(&second).is_some());
        drop(mapper);
        drop(middle);
        assert!(
            resolver
                .resolver
                .mapper
                .read()
                .unwrap()
                .get(&second)
                .is_none()
        );
        drop(late);
    }

    #[test]
    fn forgotten_parent_is_released_after_its_last_child() {
        let resolver = PathResolver::new();
        let dir = resolver.lookup(ROOT_INODE, OsStr::new("dir"), (), true);
        let child = resolver.lookup(dir, OsStr::new("child"), (), true);
        resolver.forget(dir, 1);
        assert_eq!(resolver.resolve_id(child), PathBuf::from("dir/child"));
        assert!(resolver.resolver.mapper.read().unwrap().get(&dir).is_some());
        resolver.forget(child, 1);
        let mapper = resolver.resolver.mapper.read().unwrap();
        assert!(mapper.get(&child).is_none());
        assert!(mapper.get(&dir).is_none());
    }

    #[test]
    fn forgotten_parent_drops_unreferenced_cached_children() {
        let resolver = PathResolver::new();
        let dir = resolver.lookup(ROOT_INODE, OsStr::new("dir"), (), true);
        let child = resolver.lookup(dir, OsStr::new("cached"), (), false);
        resolver.forget(dir, 1);
        let mapper = resolver.resolver.mapper.read().unwrap();
        assert!(mapper.get(&child).is_none());
        assert!(mapper.get(&dir).is_none());
    }

    #[test]
    fn overwritten_destination_survives_until_final_forget() {
        let resolver = PathResolver::new();
        let source = resolver.lookup(ROOT_INODE, OsStr::new("source"), (), true);
        let destination = resolver.lookup(ROOT_INODE, OsStr::new("destination"), (), true);
        resolver.rename(
            ROOT_INODE,
            OsStr::new("source"),
            ROOT_INODE,
            OsStr::new("destination"),
        );
        assert!(
            resolver
                .resolver
                .mapper
                .read()
                .unwrap()
                .get(&destination)
                .is_some()
        );
        resolver.forget(destination, 1);
        assert!(
            resolver
                .resolver
                .mapper
                .read()
                .unwrap()
                .get(&destination)
                .is_none()
        );
        assert_eq!(
            resolver
                .resolver
                .mapper
                .read()
                .unwrap()
                .lookup(&ROOT_INODE, OsStr::new("destination"))
                .unwrap()
                .inode,
            &source
        );
    }

    #[test]
    fn test_components_resolver() {
        let resolver = ComponentsResolver::new();

        // Test lookup and resolve_id
        let parent_ino = ROOT_INODE;
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
    }

    #[test]
    #[should_panic(expected = "Failed to resolve inode")]
    fn test_components_resolver_prune_panics_on_resolved_deleted() {
        let resolver = ComponentsResolver::new();
        let parent_ino = ROOT_INODE;
        let child_ino = resolver.lookup(parent_ino, OsStr::new("child"), (), true);
        resolver.forget(child_ino, 1);
        resolver.prune(&HashSet::new());
        resolver.resolve_id(child_ino);
    }

    #[test]
    fn test_path_resolver() {
        let resolver = PathResolver::new();

        // Test lookup and resolve_id for root
        let root_ino = ROOT_INODE;
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

        // Test forget once no further operations need the inode.
        resolver.forget(file_ino, 1);

        // Test lookup for non-existent file
        let non_existent_ino = resolver.lookup(root_ino, OsStr::new("non_existent"), (), false);
        assert_ne!(non_existent_ino.0, 0);
        let non_existent_path = resolver.resolve_id(non_existent_ino);
        assert_eq!(non_existent_path, PathBuf::from("non_existent"));
    }

    #[test]
    fn path_resolver_keeps_last_path_after_unlink() {
        let resolver = PathResolver::new();
        let inode = resolver.lookup(ROOT_INODE, OsStr::new("a.txt"), (), true);

        resolver.unlink(ROOT_INODE, OsStr::new("a.txt"));

        assert_eq!(resolver.resolve_id(inode), PathBuf::from("a.txt"));
    }

    #[test]
    fn registered_links_keep_one_inode_through_forget_and_unlink() {
        let resolver = MappedResolver::new();
        let ino = resolver.lookup(ROOT_INODE, OsStr::new("a"), (), true);
        let id = resolver.resolve_id(ino);
        assert!(resolver.link(ino, ROOT_INODE, OsStr::new("b")));
        assert_eq!(id.inode(), ino);
        assert_eq!(id.paths().len(), 2);
        assert!(id.paths().contains(&PathBuf::from("a")));
        assert!(id.paths().contains(&PathBuf::from("b")));

        resolver.forget(ino, 2);
        assert_eq!(resolver.lookup(ROOT_INODE, OsStr::new("b"), (), true), ino);
        resolver.unlink(ROOT_INODE, OsStr::new("a"));
        assert_eq!(id.paths(), vec![PathBuf::from("b")]);
        resolver.rename(ROOT_INODE, OsStr::new("b"), ROOT_INODE, OsStr::new("c"));
        assert_eq!(id.paths(), vec![PathBuf::from("c")]);
        resolver.unlink(ROOT_INODE, OsStr::new("c"));
        assert!(id.paths().is_empty());
        assert_eq!(id.inode(), ino);
        resolver.forget(ino, 1);
        assert!(id.paths().is_empty());
    }

    #[test]
    fn unrelated_lookup_does_not_claim_existing_inode() {
        let resolver = MappedResolver::new();
        let a = resolver.lookup(ROOT_INODE, OsStr::new("a"), (), true);
        let b = resolver.lookup(ROOT_INODE, OsStr::new("b"), (), true);
        assert_ne!(a, b);
    }

    #[test]
    fn forgotten_parent_still_resolves_registered_link() {
        let resolver = MappedResolver::new();
        let dir = resolver.lookup(ROOT_INODE, OsStr::new("dir"), (), true);
        let file = resolver.lookup(dir, OsStr::new("a"), (), true);
        let id = resolver.resolve_id(file);
        resolver.link(file, dir, OsStr::new("b"));
        resolver.forget(dir, 1);
        resolver.forget(file, 2);
        let paths = id.paths();
        assert!(paths.contains(&PathBuf::from("dir/a")));
        assert!(paths.contains(&PathBuf::from("dir/b")));
    }

    #[test]
    fn rename_over_other_inode_removes_only_destination_name() {
        let resolver = MappedResolver::new();
        let source = resolver.lookup(ROOT_INODE, OsStr::new("source"), (), true);
        let destination = resolver.lookup(ROOT_INODE, OsStr::new("destination"), (), true);
        let old_destination = resolver.resolve_id(destination);
        resolver.link(source, ROOT_INODE, OsStr::new("alias"));
        resolver.rename(
            ROOT_INODE,
            OsStr::new("alias"),
            ROOT_INODE,
            OsStr::new("destination"),
        );
        assert_eq!(
            resolver.lookup(ROOT_INODE, OsStr::new("destination"), (), true),
            source
        );
        assert_eq!(resolver.resolve_id(source).paths().len(), 2);
        assert!(old_destination.paths().is_empty());
    }

    #[test]
    fn test_path_resolver_back_and_forth_rename() {
        let resolver = PathResolver::new();

        // Test lookup and resolve_id for root
        let root_ino = ROOT_INODE;
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

#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::{hint::black_box, sync::Barrier, thread, time::Instant};

    // Run with:
    // cargo test --release --lib benchmark_request_paths -- --ignored --nocapture
    // EASY_FUSER_BENCH_ITERS and EASY_FUSER_BENCH_SAMPLES adjust the run size.
    #[test]
    #[ignore = "manual performance benchmark"]
    fn benchmark_request_paths() {
        let iterations = std::env::var("EASY_FUSER_BENCH_ITERS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(50_000);
        let samples = std::env::var("EASY_FUSER_BENCH_SAMPLES")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3);
        println!(
            "path,guard,threads,iterations_per_thread,sample,ns_per_resolver_op,resolver_ops_per_second"
        );
        for threads in [1, 4, 8] {
            for path in ["resolve", "lookup_forget"] {
                for sample in 0..samples {
                    for guarded in [false, true] {
                        let elapsed = run(path, guarded, threads, iterations);
                        let operations =
                            iterations * threads * if path == "lookup_forget" { 2 } else { 1 };
                        println!(
                            "{path},{guarded},{threads},{iterations},{sample},{},{}",
                            elapsed.as_nanos() / operations as u128,
                            operations as u128 * 1_000_000_000 / elapsed.as_nanos()
                        );
                    }
                }
            }
        }
    }

    fn run(path: &str, guarded: bool, threads: usize, iterations: usize) -> std::time::Duration {
        let resolver = Arc::new(PathResolver::new());
        let inode = resolver.lookup(ROOT_INODE, OsStr::new("stable"), (), true);
        let churn = path == "lookup_forget";
        let barrier = Arc::new(Barrier::new(threads + 1));
        let workers: Vec<_> = (0..threads)
            .map(|thread_id| {
                let resolver = resolver.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    let names: Vec<_> = if churn {
                        (0..iterations)
                            .map(|index| OsString::from(format!("file-{thread_id}-{index}")))
                            .collect()
                    } else {
                        Vec::new()
                    };
                    barrier.wait();
                    if !churn {
                        for _ in 0..iterations {
                            if guarded {
                                let request = RequestResolver::new(resolver.clone());
                                black_box(request.resolve_id(inode));
                            } else {
                                let request = resolver.clone();
                                black_box(request.resolve_id(inode));
                            }
                        }
                    } else {
                        for name in names {
                            if guarded {
                                let request = RequestResolver::new(resolver.clone());
                                let ino = request.lookup(ROOT_INODE, &name, (), true);
                                drop(request);
                                let request = RequestResolver::new(resolver.clone());
                                request.forget(ino, 1);
                            } else {
                                let request = resolver.clone();
                                let ino = request.lookup(ROOT_INODE, &name, (), true);
                                drop(request);
                                let request = resolver.clone();
                                request.forget(ino, 1);
                            }
                        }
                    }
                })
            })
            .collect();
        let start = Instant::now();
        barrier.wait();
        for worker in workers {
            worker.join().unwrap();
        }
        start.elapsed()
    }
}
