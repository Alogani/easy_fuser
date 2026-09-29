use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::hash::Hash;
use std::sync::Arc;

use crate::types::{Inode, InodeExt, ROOT_INODE};
use std::sync::atomic::{AtomicU64, Ordering};

/// Assigns inode numbers and remembers which directory entries name each inode.
///
/// A normal insert creates one inode for one `(parent, name)` pair. After the
/// backing filesystem creates a hard link, call [`Self::link`] to attach its
/// second name to the same inode. `Data` remains the caller's own data and is
/// not used to detect links.
///
/// ```
/// use easy_fuser::inode_mapping::InodeMapper;
/// use easy_fuser::types::ROOT_INODE;
/// use std::ffi::OsStr;
///
/// let mut mapper = InodeMapper::new(());
/// let inode = mapper.insert_child(&ROOT_INODE, "a.txt".into(), |_| ()).unwrap();
/// mapper.link(&inode, &ROOT_INODE, "b.txt".into()).unwrap();
/// assert_eq!(mapper.lookup(&ROOT_INODE, OsStr::new("b.txt")).unwrap().inode, &inode);
/// assert_eq!(mapper.resolve(&inode).unwrap().len(), 2);
/// ```
///
/// Handlers using `MappedInode` get this behavior automatically when their
/// `link` operation succeeds.
pub struct InodeMapper<Data> {
    data: InodeData<Data>,
    next_inode: Inode,
}

struct InodeData<T> {
    /// Caller data for each inode. Names live in the separate link indices.
    inodes: HashMap<Inode, T>,
    /// Forward index: (parent, name) -> inode.
    children: HashMap<Inode, HashMap<OsStringWrapper, Inode>>,
    /// Reverse index: inode -> all (parent, name) entries.
    /// Both indices share each name's Arc rather than copying its text.
    links: HashMap<Inode, HashMap<Inode, HashSet<OsStringWrapper>>>,
}

pub trait HasLookupCount {
    fn lookup_count(&self) -> &AtomicU64;
    fn lookup_count_mut(&mut self) -> &mut AtomicU64;
}

pub struct ValueCreatorParams<'a, T> {
    pub new_inode: &'a Inode,
    pub parent: &'a Inode,
    pub child_name: &'a OsStr,
    pub existing_data: Option<&'a T>,
}

pub struct LookupResult<'a, T> {
    pub inode: &'a Inode,
    pub name: &'a Arc<OsString>,
    pub data: &'a T,
}

pub struct InodeInfo<'a, T> {
    /// All names currently registered for this inode, collected by `get()`.
    /// Empty after its last unlink.
    pub links: Vec<(&'a Inode, &'a Arc<OsString>)>,
    pub data: &'a T,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InsertError {
    ParentNotFound,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RenameError {
    NotFound,
    ParentNotFound,
    NewParentNotFound,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LinkError {
    InodeNotFound,
    ParentNotFound,
    NameExists,
}

/// A wrapper around `Arc<OsString>` for efficient storage and comparison in hash maps.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct OsStringWrapper(Arc<OsString>);

impl AsRef<Arc<OsString>> for OsStringWrapper {
    fn as_ref(&self) -> &Arc<OsString> {
        &self.0
    }
}

impl Borrow<OsStr> for OsStringWrapper {
    fn borrow(&self) -> &OsStr {
        self.0.as_os_str()
    }
}

impl<Data: Send + Sync + 'static> InodeMapper<Data> {
    /// Creates a new `InodeMapper` instance with the root inode initialized.
    ///
    /// This function initializes the `InodeMapper` with an empty structure and sets up the root inode
    /// with the provided data. The root inode is represented by an empty path.
    pub fn new(data: Data) -> Self {
        let mut result = InodeMapper {
            data: InodeData {
                inodes: HashMap::new(),
                children: HashMap::new(),
                links: HashMap::new(),
            },
            next_inode: ROOT_INODE.add_one(),
        };
        result.data.inodes.insert(ROOT_INODE, data);
        result
    }

    pub fn get_root_inode(&self) -> Inode {
        ROOT_INODE
    }

    /// A private method that inserts a child inode into the InodeMapper, even if the parent doesn't exist.
    ///
    /// This function creates a new inode or updates an existing one, associating it with the given parent and child name. It uses a value_creator function to generate or update the data associated with the inode.
    ///
    /// Note: This method doesn't check if the parent exists, which can lead to inconsistencies if used incorrectly. It's primarily intended for internal use or in scenarios where the parent's existence is guaranteed.
    ///
    /// # Behavior:
    /// - If the child doesn't exist, a new inode is created with a unique ID.
    /// - If the child already exists, its data is updated using the value_creator function.
    /// - The value_creator function is called with the inode, parent, child name, and existing data (if any) as arguments.
    ///
    /// Caveat: This method may create orphaned inodes if used with non-existent parents. Use with caution.
    fn insert_child_unchecked<F>(
        &mut self,
        parent: &Inode,
        child: OsString,
        value_creator: F,
    ) -> Inode
    where
        F: Fn(ValueCreatorParams<Data>) -> Data,
    {
        if let Some(inode) = self
            .data
            .children
            .get(parent)
            .and_then(|children| children.get(child.as_os_str()))
            .copied()
        {
            let inode_value = self.data.inodes.get_mut(&inode).unwrap();
            *inode_value = value_creator(ValueCreatorParams {
                parent,
                new_inode: &inode,
                child_name: child.as_os_str(),
                existing_data: Some(inode_value),
            });
            return inode;
        }

        let inode = self.next_inode;
        self.next_inode = inode.add_one();
        let child = OsStringWrapper(Arc::new(child));
        self.data
            .children
            .entry(*parent)
            .or_default()
            .insert(child.clone(), inode);
        self.data.inodes.insert(
            inode,
            value_creator(ValueCreatorParams {
                parent,
                new_inode: &inode,
                child_name: child.as_ref(),
                existing_data: None,
            }),
        );
        self.data
            .links
            .insert(inode, HashMap::from([(*parent, HashSet::from([child]))]));
        inode
    }

    /// Safely inserts a child inode into the InodeMapper.
    ///
    /// This method checks if the parent exists before inserting the child. It uses a value_creator
    /// function to generate the data associated with the new inode.
    ///
    /// # Behavior
    /// - Returns Err(InsertError::ParentNotFound) if the parent doesn't exist.
    /// - If successful, returns Ok(Inode) with the newly created or existing child inode.
    ///
    /// The value_creator function is called with the new inode, parent inode, child name,
    /// and existing data (if any) as arguments.
    pub fn insert_child<F>(
        &mut self,
        parent: &Inode,
        child: OsString,
        value_creator: F,
    ) -> Result<Inode, InsertError>
    where
        F: Fn(ValueCreatorParams<Data>) -> Data,
    {
        if !self.data.inodes.contains_key(parent) {
            return Err(InsertError::ParentNotFound);
        }

        Ok(self.insert_child_unchecked(parent, child, value_creator))
    }

    /// Inserts multiple children into the InodeMapper for a given parent inode.
    ///
    /// This method efficiently inserts multiple children at once, optimizing memory allocation
    /// for the parent's children HashMap. It checks if the parent exists before insertion.
    ///
    /// # Behavior
    /// - Returns `Err(InsertError::ParentNotFound)` if the parent doesn't exist.
    /// - If successful, returns `Ok(Vec<Inode>)` containing the newly created or existing child inodes.
    ///
    /// # Performance
    /// Optimizes memory allocation by reserving space in the parent's children HashMap based on
    /// the number of new children to be inserted.
    pub fn insert_children<F>(
        &mut self,
        parent: &Inode,
        children: Vec<(OsString, F)>,
    ) -> Result<Vec<Inode>, InsertError>
    where
        F: Fn(ValueCreatorParams<Data>) -> Data,
    {
        if !self.data.inodes.contains_key(parent) {
            return Err(InsertError::ParentNotFound);
        }

        // Reserve space in the parent's children HashMap
        if let Some(parent_children) = self.data.children.get_mut(parent) {
            if parent_children.is_empty() {
                parent_children.reserve(children.len());
            } else if children.len() > parent_children.len() {
                parent_children.reserve(children.len() - parent_children.len());
            }
        } else {
            // If the parent doesn't exist yet, create it with the right capacity
            self.data
                .children
                .insert(parent.clone(), HashMap::with_capacity(children.len()));
        }

        Ok(children
            .into_iter()
            .map(|(child_name, value_creator)| {
                self.insert_child_unchecked(parent, child_name, value_creator)
            })
            .collect())
    }

    /// Batch inserts multiple entries into the InodeMapper, creating missing parent directories as needed.
    ///
    /// This method efficiently handles the insertion of multiple entries, potentially with nested paths.
    /// It sorts entries by path length to ensure parent directories are created before their children.
    ///
    /// # Behavior
    /// - Creates missing parent directories using the default_parent_creator function. (data field will always be null)
    /// - Inserts entries using the provided value_creator function.
    /// - Returns Err(InsertError::ParentNotFound) if the initial parent inode doesn't exist.
    ///
    /// # Note
    /// Expects each entry's path to include the entry name as the last element.
    ///
    /// # Caveats
    /// If the closures are not defined in same scope, ther emight be a compiler error concerning lifetimes (eg: implementation of `Fn` is not general enough)
    /// To resolve this problem, always fully qualify the argumentsof the closure (eg: `|my_data: ValueCreatorParams<MyType>| {}` and not `|my_data| {}`)
    pub fn batch_insert<F, G>(
        &mut self,
        parent: &Inode,
        entries: Vec<(Vec<OsString>, F)>,
        default_parent_creator: G,
    ) -> Result<(), InsertError>
    where
        F: Fn(ValueCreatorParams<Data>) -> Data,
        G: Fn(ValueCreatorParams<Data>) -> Data,
    {
        if !self.data.inodes.contains_key(parent) {
            return Err(InsertError::ParentNotFound);
        }

        // Sort entries by path length to ensure parents are created first
        let mut sorted_entries = entries;
        sorted_entries.sort_by_key(|f| f.0.len());

        let mut path_cache: HashMap<Vec<OsString>, Inode> = HashMap::new();
        path_cache.insert(vec![], parent.clone());

        for (mut path, value_creator) in sorted_entries {
            let name = path.pop().expect("Name should be provided");
            let parent_inode =
                self.ensure_path_exists(&mut path_cache, &path, &default_parent_creator);
            self.insert_child_unchecked(&parent_inode, name, value_creator);
        }

        Ok(())
    }

    fn ensure_path_exists(
        &mut self,
        path_cache: &mut HashMap<Vec<OsString>, Inode>,
        path: &[OsString],
        default_parent_creator: &impl Fn(ValueCreatorParams<Data>) -> Data,
    ) -> Inode {
        let mut current_inode = path_cache[&vec![]].clone();
        for (i, component) in path.iter().enumerate() {
            let current_path = &path[..=i];
            if let Some(inode) = path_cache.get(current_path) {
                current_inode = inode.clone();
            } else {
                let new_inode = if let Some(children) = self.data.children.get_mut(&current_inode) {
                    if let Some(child_inode) = children.get(component.as_os_str()) {
                        child_inode.clone()
                    } else {
                        self.insert_child_unchecked(
                            &current_inode,
                            component.clone(),
                            |mut value_creator_params| {
                                value_creator_params.existing_data = None;
                                default_parent_creator(value_creator_params)
                            },
                        )
                    }
                } else {
                    self.insert_child_unchecked(
                        &current_inode,
                        component.clone(),
                        |mut value_creator_params| {
                            value_creator_params.existing_data = None;
                            default_parent_creator(value_creator_params)
                        },
                    )
                };
                path_cache.insert(current_path.to_vec(), new_inode.clone());
                current_inode = new_inode;
            }
        }
        current_inode
    }

    /// Resolves one remembered path to an inode for single-path handlers.
    ///
    /// Stops after finding one path. The result is in leaf-to-root order for
    /// the legacy component resolver. An inode with no path returns an empty vector.
    pub fn resolve_first(&self, inode: &Inode) -> Option<Vec<OsString>> {
        if !self.data.inodes.contains_key(inode) {
            return None;
        }
        let mut path = self
            .resolve_first_inner(inode, &mut HashSet::new())
            .unwrap_or_default();
        path.reverse();
        Some(path)
    }

    fn resolve_first_inner(
        &self,
        inode: &Inode,
        visiting: &mut HashSet<Inode>,
    ) -> Option<Vec<OsString>> {
        if *inode == ROOT_INODE {
            return Some(Vec::new());
        }
        if !visiting.insert(*inode) {
            return None;
        }
        let path = self.data.links.get(inode).and_then(|links| {
            links.iter().find_map(|(parent, names)| {
                let name = names.iter().next()?;
                let mut path = self.resolve_first_inner(parent, visiting)?;
                path.push((**name.as_ref()).clone());
                Some(path)
            })
        });
        visiting.remove(inode);
        path
    }

    pub fn get(&self, inode: &Inode) -> Option<InodeInfo<'_, Data>> {
        self.data.inodes.get(inode).map(|inode_value| InodeInfo {
            links: self
                .data
                .links
                .get(inode)
                .into_iter()
                .flat_map(|links| links.iter())
                .flat_map(|(parent, names)| names.iter().map(move |name| (parent, name.as_ref())))
                .collect(),
            data: inode_value,
        })
    }

    /// Returns every known path to an inode, with components in root-to-leaf order.
    /// An existing inode with no directory entries returns an empty vector.
    pub fn resolve(&self, inode: &Inode) -> Option<Vec<Vec<OsString>>> {
        if !self.data.inodes.contains_key(inode) {
            return None;
        }
        if *inode == ROOT_INODE {
            return Some(vec![Vec::new()]);
        }
        self.resolve_all_inner(inode, &mut HashSet::new())
    }

    fn resolve_all_inner(
        &self,
        inode: &Inode,
        visiting: &mut HashSet<Inode>,
    ) -> Option<Vec<Vec<OsString>>> {
        if *inode == ROOT_INODE {
            return Some(vec![Vec::new()]);
        }
        if !visiting.insert(*inode) {
            return Some(Vec::new());
        }
        if !self.data.inodes.contains_key(inode) {
            visiting.remove(inode);
            return None;
        }
        let mut paths = Vec::new();
        for (parent, names) in self.data.links.get(inode).into_iter().flatten() {
            if let Some(parent_paths) = self.resolve_all_inner(parent, visiting) {
                for parent_path in parent_paths {
                    for name in names {
                        let mut path = parent_path.clone();
                        path.push((**name.as_ref()).clone());
                        paths.push(path);
                    }
                }
            }
        }
        visiting.remove(inode);
        Some(paths)
    }

    /// Returns mutable access to the data stored for an inode.
    pub fn get_data_mut(&mut self, inode: &Inode) -> Option<&mut Data> {
        self.data.inodes.get_mut(inode)
    }

    // Retrieves all children of a given parent inode.
    ///
    /// # Note
    /// - Does not check if the parent inode exists.
    /// - Returns an empty vector if the parent has no children or doesn't exist.
    pub fn get_children(&self, parent: &Inode) -> Vec<(&Arc<OsString>, &Inode)> {
        self.data
            .children
            .get(parent)
            .map(|children| {
                children
                    .iter()
                    .map(|(name, inode)| (name.as_ref(), inode))
                    .collect()
            })
            .unwrap_or(vec![])
    }

    /// Looks up a child inode by its parent inode and name
    pub fn lookup(&self, parent: &Inode, name: &OsStr) -> Option<LookupResult<'_, Data>> {
        self.data
            .children
            .get(parent)
            .and_then(|children| children.get_key_value(name))
            .map(|(name, child_inode)| {
                let inode_value = self.data.inodes.get(child_inode).unwrap();
                LookupResult {
                    inode: child_inode,
                    name: name.as_ref(),
                    data: inode_value,
                }
            })
    }

    /// Registers a new hard-link name for an existing inode. Call this only
    /// after the backing filesystem has successfully created the link.
    pub fn link(&mut self, inode: &Inode, parent: &Inode, name: OsString) -> Result<(), LinkError> {
        if !self.data.inodes.contains_key(inode) {
            return Err(LinkError::InodeNotFound);
        }
        if !self.data.inodes.contains_key(parent) {
            return Err(LinkError::ParentNotFound);
        }
        if self
            .data
            .children
            .get(parent)
            .is_some_and(|children| children.contains_key(name.as_os_str()))
        {
            return Err(LinkError::NameExists);
        }
        let name = OsStringWrapper(Arc::new(name));
        self.data
            .children
            .entry(*parent)
            .or_default()
            .insert(name.clone(), *inode);
        self.data
            .links
            .entry(*inode)
            .or_default()
            .entry(*parent)
            .or_default()
            .insert(name);
        Ok(())
    }

    /// Removes one name while keeping the inode alive for remaining links and
    /// outstanding FUSE references. Returns the inode that lost the name.
    pub fn unlink(&mut self, parent: &Inode, name: &OsStr) -> Option<Inode> {
        let children = self.data.children.get_mut(parent)?;
        let inode = children.remove(name)?;
        if children.is_empty() {
            self.data.children.remove(parent);
        }
        let links = self.data.links.get_mut(&inode)?;
        if let Some(names) = links.get_mut(parent) {
            names.remove(name);
            if names.is_empty() {
                links.remove(parent);
            }
        }
        if links.is_empty() {
            self.data.links.remove(&inode);
        }
        Some(inode)
    }

    pub fn has_links(&self, inode: &Inode) -> bool {
        self.data
            .links
            .get(inode)
            .is_some_and(|links| !links.is_empty())
    }

    /// Renames a child inode from one parent to another
    pub fn rename(
        &mut self,
        parent: &Inode,
        oldname: &OsStr,
        newparent: &Inode,
        newname: OsString,
    ) -> Result<Option<(Inode, Data)>, RenameError> {
        if !self.data.inodes.contains_key(parent) {
            return Err(RenameError::ParentNotFound);
        }
        if !self.data.inodes.contains_key(newparent) {
            return Err(RenameError::NewParentNotFound);
        }
        let child_inode = self
            .data
            .children
            .get(parent)
            .ok_or(RenameError::NotFound)
            .and_then(|children| children.get(oldname).copied().ok_or(RenameError::NotFound))?;
        if *parent == *newparent && oldname == newname.as_os_str() {
            return Ok(None);
        }
        // Renaming one hard link over another name of the same inode is a no-op.
        if self
            .data
            .children
            .get(newparent)
            .and_then(|c| c.get(newname.as_os_str()))
            == Some(&child_inode)
        {
            return Ok(None);
        }
        self.unlink(parent, oldname)
            .expect("source name was checked");
        self.unlink(newparent, newname.as_os_str());
        let newname = OsStringWrapper(Arc::new(newname));
        self.data
            .children
            .entry(*newparent)
            .or_default()
            .insert(newname.clone(), child_inode);
        self.data
            .links
            .entry(child_inode)
            .or_default()
            .entry(*newparent)
            .or_default()
            .insert(newname);
        Ok(None)
    }

    /// Swaps two existing directory entries after an exchange rename succeeds.
    pub fn exchange(
        &mut self,
        parent: &Inode,
        name: &OsStr,
        newparent: &Inode,
        newname: &OsStr,
    ) -> Result<(), RenameError> {
        let left = self
            .lookup(parent, name)
            .ok_or(RenameError::NotFound)?
            .inode
            .to_owned();
        let right = self
            .lookup(newparent, newname)
            .ok_or(RenameError::NotFound)?
            .inode
            .to_owned();
        if *parent == *newparent && name == newname {
            return Ok(());
        }
        self.unlink(parent, name);
        self.unlink(newparent, newname);
        for (inode, target_parent, target_name) in
            [(left, *newparent, newname), (right, *parent, name)]
        {
            let target_name = OsStringWrapper(Arc::new(target_name.to_os_string()));
            self.data
                .children
                .entry(target_parent)
                .or_default()
                .insert(target_name.clone(), inode);
            self.data
                .links
                .entry(inode)
                .or_default()
                .entry(target_parent)
                .or_default()
                .insert(target_name);
        }
        Ok(())
    }

    /// Removes an inode and its associated data from the `InodeMapper`.
    ///
    /// This function removes the specified inode from both the `inodes` and `children` maps.
    /// It also cleans up empty parent entries in the `children` map.
    ///
    /// **Note:** This operation will cascade to child inodes. If the removed inode
    /// has children, they will be removed from the data structure.
    ///
    /// **Behavior:**
    /// - Panics if we intend to remove ROOT in debug build
    /// - If the inode doesn't exist, the function does nothing.
    /// - If the parent's children map becomes empty after removal, the parent entry
    ///   is also removed from the `children` map to conserve memory.
    pub fn remove(&mut self, inode: &Inode) -> Option<Data> {
        #[cfg(debug_assertions)]
        if *inode == ROOT_INODE {
            panic!("Cannot remove ROOT");
        }
        if let Some(inode_value) = self.data.inodes.remove(inode) {
            for (parent, names) in self.data.links.remove(inode).unwrap_or_default() {
                if let Some(parent_children) = self.data.children.get_mut(&parent) {
                    for name in &names {
                        parent_children.remove(name);
                    }
                    if parent_children.is_empty() {
                        self.data.children.remove(&parent);
                    }
                }
            }
            if let Some(children) = self.data.children.remove(inode) {
                for (name, child_inode) in children {
                    if let Some(links) = self.data.links.get_mut(&child_inode) {
                        if let Some(names) = links.get_mut(inode) {
                            names.remove(&name);
                            if names.is_empty() {
                                links.remove(inode);
                            }
                        }
                    }
                    if !self.has_links(&child_inode) {
                        self.remove(&child_inode);
                    }
                }
            }
            Some(inode_value)
        } else {
            None
        }
    }
}

impl<Data> InodeMapper<Data>
where
    Data: HasLookupCount + Send + Sync + 'static,
{
    pub fn prune(&mut self, keep: &HashSet<Vec<OsString>>) {
        loop {
            let to_remove: Vec<_> = self
                .data
                .inodes
                .iter()
                .filter_map(|(inode, value)| {
                    if *inode == ROOT_INODE
                        || value.lookup_count().load(Ordering::SeqCst) != 0
                        || !self.get_children(inode).is_empty()
                    {
                        return None;
                    }
                    let path = self.resolve_first(inode)?;
                    let path: Vec<_> = path.into_iter().rev().collect();
                    (!keep.contains(&path)).then_some(*inode)
                })
                .collect();
            if to_remove.is_empty() {
                break;
            }
            for inode in to_remove {
                self.remove(&inode);
            }
        }
    }
}

impl HasLookupCount for AtomicU64 {
    fn lookup_count(&self) -> &AtomicU64 {
        self
    }

    fn lookup_count_mut(&mut self) -> &mut AtomicU64 {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashSet;
    use std::ffi::OsString;

    use crate::types::ROOT_INODE;

    #[test]
    fn test_insert_child_returns_old_inode() {
        let mut mapper = InodeMapper::new(0);
        let root = mapper.get_root_inode();
        let child_name = OsString::from("child");

        // Insert the first child
        let first_child_inode = fuser::INodeNo(2);
        assert_eq!(
            mapper.insert_child(&root, child_name.clone(), |value_creator_params| {
                assert!(value_creator_params.existing_data.is_none());
                42
            }),
            Ok(first_child_inode.clone())
        );

        // Insert a child with the same name
        assert_eq!(
            mapper.insert_child(&root, child_name.clone(), |value_creator_params| {
                assert_eq!(value_creator_params.existing_data, Some(&42));
                84
            }),
            Ok(first_child_inode.clone())
        );

        // Verify that the child was indeed replaced
        let lookup_result = mapper.lookup(&root, child_name.as_os_str());
        assert!(lookup_result.is_some());
        assert_eq!(*lookup_result.unwrap().data, 84);
    }

    #[test]
    fn test_insert_multiple_children() {
        let mut mapper = InodeMapper::new(0);
        let children: Vec<(OsString, Box<dyn Fn(ValueCreatorParams<u32>) -> u32>)> = vec![
            (OsString::from("child1"), Box::new(|_| 10)),
            (OsString::from("child2"), Box::new(|_| 20)),
            (OsString::from("child3"), Box::new(|_| 30)),
        ];

        let result = mapper.insert_children(&ROOT_INODE, children);

        assert!(result.is_ok());
        let inserted_inodes = result.unwrap();
        assert_eq!(inserted_inodes.len(), 3);

        for (i, inode) in inserted_inodes.iter().enumerate() {
            let child_name = OsString::from(format!("child{}", i + 1));
            let child_value = mapper.lookup(&ROOT_INODE, &child_name).unwrap();
            assert_eq!(child_value.inode, inode);
            assert_eq!(child_value.name.as_os_str(), &child_name);
            assert_eq!(*child_value.data, (i as u32 + 1) * 10);
        }
    }

    #[test]
    fn test_batch_insert_large_entries_varying_depths() {
        let mut mapper = InodeMapper::new(0);
        let mut entries = Vec::new();
        let mut expected_inodes = HashSet::new();

        const FILE_COUNT: usize = 50;
        // Create a large number of entries with varying depths
        for i in 0..FILE_COUNT as u64 {
            let depth = i % 5; // Vary depth from 0 to 4
            let mut path = Vec::new();
            for j in 0..depth {
                path.push(OsString::from(format!("dir_{}", j)));
            }
            path.push(OsString::from(format!("file_{}", i)));
            entries.push((path, move |_: ValueCreatorParams<u64>| i));
            expected_inodes.insert(fuser::INodeNo(i + 2)); // Start from 2 to avoid conflict with root_inode
        }

        // Perform batch insert
        let result = mapper.batch_insert(&ROOT_INODE, entries, |_: ValueCreatorParams<u64>| 0);

        // Verify results
        assert!(result.is_ok(), "Batch insert should succeed");

        // Check if all inserted inodes exist
        for i in 2..=(FILE_COUNT as u64 + 1) {
            let inode = fuser::INodeNo(i);
            assert!(mapper.get(&inode).is_some(), "{:?} should exist", inode);
        }

        // Verify the structure for a few sample paths
        let sample_paths = vec![
            vec!["file_0"],
            vec!["dir_0", "file_1"],
            vec!["dir_0", "dir_1", "file_2"],
            vec!["dir_0", "dir_1", "dir_2", "file_3"],
            vec!["dir_0", "dir_1", "dir_2", "dir_3", "file_4"],
        ];

        for (i, path) in sample_paths.iter().enumerate() {
            let mut current_inode = ROOT_INODE.clone();
            for (j, component) in path.iter().enumerate() {
                let lookup_result = mapper.lookup(&current_inode, OsStr::new(component));
                assert!(
                    lookup_result.is_some(),
                    "Failed to find {} in path {:?}",
                    component,
                    path
                );
                let lookup_result_unwraped = lookup_result.unwrap();
                if j == path.len() - 1 {
                    assert_eq!(
                        *lookup_result_unwraped.data, i as u64,
                        "Incorrect data for file {}",
                        i
                    );
                }
                current_inode = lookup_result_unwraped.inode.clone();
            }
        }
    }

    #[test]
    fn test_resolve_inode_to_full_path() {
        let mut mapper = InodeMapper::new(());

        let dir_inode = mapper
            .insert_child(&mapper.get_root_inode(), OsString::from("dir"), |_| ())
            .unwrap();
        let file_inode = mapper
            .insert_child(&dir_inode, OsString::from("file.txt"), |_| ())
            .unwrap();

        // Resolve the file inode
        let path = mapper.resolve_first(&file_inode).unwrap();

        // Check the resolved path (it should be in reverse order)
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], OsString::from("file.txt"));
        assert_eq!(path[1], OsString::from("dir"));

        // Resolve the root inode (should be empty)
        let root_path = mapper.resolve_first(&ROOT_INODE).unwrap();
        assert!(root_path.is_empty());

        // Try to resolve a non-existent inode
        assert!(mapper.resolve(&fuser::INodeNo(999)).is_none());
    }

    #[test]
    fn test_resolve_invalid_inode() {
        let mapper = InodeMapper::new(0);
        let invalid_inode = fuser::INodeNo(999);

        // Attempt to resolve an invalid inode
        let result = mapper.resolve(&invalid_inode);

        // Assert that the result is None
        assert!(
            result.is_none(),
            "Resolving an invalid inode should return None"
        );
    }

    #[test]
    fn resolve_first_uses_a_path_with_reachable_parents() {
        let mut mapper = InodeMapper::new(());
        let old_parent = mapper
            .insert_child(&ROOT_INODE, OsString::from("old"), |_| ())
            .unwrap();
        let new_parent = mapper
            .insert_child(&ROOT_INODE, OsString::from("new"), |_| ())
            .unwrap();
        let file = mapper
            .insert_child(&old_parent, OsString::from("file"), |_| ())
            .unwrap();
        mapper
            .link(&file, &new_parent, OsString::from("file"))
            .unwrap();
        mapper.unlink(&ROOT_INODE, OsStr::new("old"));

        assert_eq!(
            mapper.resolve_first(&file),
            Some(vec![OsString::from("file"), OsString::from("new")])
        );
    }

    #[test]
    fn test_rename_child_inode() {
        let mut mapper = InodeMapper::new(());
        let root = mapper.get_root_inode();

        // Insert initial structure
        let parent1 = mapper
            .insert_child(&root, OsString::from("parent1"), |_| ())
            .unwrap();
        let parent2 = mapper
            .insert_child(&root, OsString::from("parent2"), |_| ())
            .unwrap();
        let child = mapper
            .insert_child(&parent1, OsString::from("old_name"), |_| ())
            .unwrap();
        mapper
            .insert_child(&parent2, OsString::from("dummy"), |_| ())
            .unwrap();

        // Perform rename
        let result = mapper.rename(
            &parent1,
            OsStr::new("old_name"),
            &parent2,
            OsString::from("new_name"),
        );

        // Assert successful rename
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        // Verify new location
        let renamed_child = mapper.lookup(&parent2, OsStr::new("new_name"));
        assert!(renamed_child.is_some());
        assert_eq!(renamed_child.unwrap().inode, &child);

        // Verify old location is empty
        assert!(mapper.lookup(&parent1, OsStr::new("old_name")).is_none());

        // Verify inode data is updated
        assert_eq!(
            mapper.resolve(&child).unwrap(),
            vec![vec![OsString::from("parent2"), OsString::from("new_name")]]
        );
    }

    #[test]
    fn test_should_not_prematurely_purge_old_inode_after_renaming() {
        // Data fields of all inodes in this test are 1 to simulate reflection of the FUSE inode refcount
        let mut mapper = InodeMapper::new(1u64);
        let root = mapper.get_root_inode();

        let parent1 = mapper
            .insert_child(&root, OsString::from("parent1"), |_| 1)
            .unwrap();
        let parent2 = mapper
            .insert_child(&root, OsString::from("parent2"), |_| 1)
            .unwrap();
        let child1 = mapper
            .insert_child(&parent1, OsString::from("child1"), |_| 1)
            .unwrap();
        let child2 = mapper
            .insert_child(&parent2, OsString::from("child2"), |_| 1)
            .unwrap();

        // Rename child1 to child2
        mapper
            .rename(
                &parent1,
                OsStr::new("child1"),
                &parent2,
                OsString::from("child2"),
            )
            .expect("should be able to insert inode");
        assert!(
            mapper.get(&child1).is_some(),
            "first inode should be present"
        );
        assert!(
            mapper
                .get(&child1)
                .unwrap()
                .links
                .iter()
                .any(|(parent, name)| **parent == parent2
                    && name.as_os_str() == OsStr::new("child2")),
            "first inode should point to parent2 as parent"
        );
        assert!(
            mapper
                .get_children(&parent2)
                .contains(&(&Arc::new(OsString::from("child2")), &child1)),
            "first inode should be in parent2's child node list"
        );
        assert!(
            mapper.get(&child2).is_some(),
            "second inode must be present as an orphaned inode but not removed immediately"
        );
    }

    #[test]
    fn test_rename_child_inode_into_empty_dir_inode() {
        let mut mapper = InodeMapper::new(());
        let root = mapper.get_root_inode();

        // Insert initial structure
        let parent1 = mapper
            .insert_child(&root, OsString::from("parent1"), |_| ())
            .unwrap();
        let parent2 = mapper
            .insert_child(&parent1, OsString::from("parent2"), |_| ())
            .unwrap();
        let child = mapper
            .insert_child(&root, OsString::from("test_name"), |_| ())
            .unwrap();

        // Perform rename
        let result = mapper.rename(
            &root,
            OsStr::new("test_name"),
            &parent2,
            OsString::from("test_name"),
        );

        // Assert successful rename
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        // Verify new location
        let renamed_child = mapper.lookup(&parent2, OsStr::new("test_name"));
        assert!(renamed_child.is_some());
        assert_eq!(renamed_child.unwrap().inode, &child);

        // Verify old location is empty
        assert!(mapper.lookup(&root, OsStr::new("test_name")).is_none());

        // Verify inode data is updated
        assert_eq!(mapper.get(&child).unwrap().links.len(), 1);
        assert_eq!(
            mapper.resolve(&child).unwrap(),
            vec![vec![
                OsString::from("parent1"),
                OsString::from("parent2"),
                OsString::from("test_name")
            ]]
        );

        // Perform rename back to original path
        let result = mapper.rename(
            &parent2,
            OsStr::new("test_name"),
            &root,
            OsString::from("test_name"),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        // Verify new location
        let renamed_child = mapper.lookup(&root, OsStr::new("test_name"));
        assert!(renamed_child.is_some());
        assert_eq!(renamed_child.unwrap().inode, &child);

        // Verify old location is empty
        assert!(mapper.lookup(&parent2, OsStr::new("test_name")).is_none());

        // Verify inode data is updated
        assert_eq!(mapper.get(&child).unwrap().links.len(), 1);
        assert_eq!(
            mapper.resolve(&child).unwrap(),
            vec![vec![OsString::from("test_name")]]
        );
    }

    #[test]
    fn test_rename_non_existent_child() {
        let mut mapper = InodeMapper::new(0);

        // Insert parent inodes
        let root = mapper.get_root_inode();
        let parent = mapper
            .insert_child(&root, OsString::from("parent"), |_| 1)
            .unwrap();
        let newparent = mapper
            .insert_child(&root, OsString::from("newparent"), |_| 2)
            .unwrap();

        // Attempt to rename a non-existent child
        let result = mapper.rename(
            &parent,
            OsStr::new("non_existent"),
            &newparent,
            OsString::from("new_name"),
        );

        assert!(matches!(result, Err(RenameError::NotFound)));
    }

    #[test]
    fn test_remove_cascading() {
        let mut mapper = InodeMapper::new(());
        let child1 = fuser::INodeNo(2);
        let child2 = fuser::INodeNo(3);
        let grandchild1 = fuser::INodeNo(4);
        let grandchild2 = fuser::INodeNo(5);
        let great_grandchild = fuser::INodeNo(6);

        // Create a deeper nested structure
        mapper
            .insert_child(&ROOT_INODE, OsString::from("child1"), |_| ())
            .unwrap();
        mapper
            .insert_child(&ROOT_INODE, OsString::from("child2"), |_| ())
            .unwrap();
        mapper
            .insert_child(&child1, OsString::from("grandchild1"), |_| ())
            .unwrap();
        mapper
            .insert_child(&child1, OsString::from("grandchild2"), |_| ())
            .unwrap();
        mapper
            .insert_child(&grandchild1, OsString::from("great_grandchild"), |_| ())
            .unwrap();

        // Remove child1, which should cascade to all its descendants
        mapper.remove(&child1);

        // Check that child1 and all its descendants are removed
        assert!(mapper.get(&child1).is_none());
        assert!(mapper.get(&grandchild1).is_none());
        assert!(mapper.get(&grandchild2).is_none());
        assert!(mapper.get(&great_grandchild).is_none());

        // Check that child2 still exists
        assert!(mapper.get(&child2).is_some());

        // Check that root still has only child2
        assert_eq!(mapper.get_children(&ROOT_INODE).len(), 1);
        assert!(mapper.lookup(&ROOT_INODE, OsStr::new("child2")).is_some());

        // Remove child2
        mapper.remove(&child2);

        // Check that child2 is removed
        assert!(mapper.get(&child2).is_none());

        // Check that root still exists but has no children
        assert!(mapper.get(&ROOT_INODE).is_some());
        assert!(mapper.get_children(&ROOT_INODE).is_empty());

        // Verify that only ROOT_INODE remains in the inodes map
        assert_eq!(mapper.get_children(&ROOT_INODE).len(), 0);
        assert!(mapper.get(&ROOT_INODE).is_some());
    }

    #[test]
    fn test_prune_inodes() {
        let mut mapper = InodeMapper::new(AtomicU64::new(1)); // Root starts with 1

        // Add a child with 0 refcount
        let child_ino = mapper
            .insert_child(&ROOT_INODE, OsString::from("child"), |_| AtomicU64::new(0))
            .unwrap();

        // Prune with empty keep set
        let keep = HashSet::new();
        mapper.prune(&keep);

        // Child should be gone
        assert!(mapper.get(&child_ino).is_none());

        // Add it back
        let child_ino = mapper
            .insert_child(&ROOT_INODE, OsString::from("child"), |_| AtomicU64::new(0))
            .unwrap();

        // Prune with keep set containing the path
        let mut keep = HashSet::new();
        keep.insert(vec![OsString::from("child")]);
        mapper.prune(&keep);

        // Child should remain because it's in the keep set
        assert!(mapper.get(&child_ino).is_some());

        // Increment refcount
        mapper
            .get(&child_ino)
            .unwrap()
            .data
            .fetch_add(1, Ordering::SeqCst);

        // Prune with empty keep set
        let keep = HashSet::new();
        mapper.prune(&keep);

        // Child should remain because refcount > 0
        assert!(mapper.get(&child_ino).is_some());
    }

    #[test]
    fn prune_keeps_referenced_descendants_of_forgotten_directory() {
        let mut mapper = InodeMapper::new(AtomicU64::new(0));
        let dir = mapper
            .insert_child(&ROOT_INODE, "dir".into(), |_| AtomicU64::new(0))
            .unwrap();
        let file = mapper
            .insert_child(&dir, "file".into(), |_| AtomicU64::new(1))
            .unwrap();
        mapper.prune(&HashSet::new());
        assert!(mapper.get(&dir).is_some());
        assert!(mapper.get(&file).is_some());
        assert_eq!(
            mapper.resolve_first(&file),
            Some(vec![OsString::from("file"), OsString::from("dir")])
        );

        mapper.get(&file).unwrap().data.store(0, Ordering::SeqCst);
        mapper.prune(&HashSet::new());
        assert!(mapper.get(&file).is_none());
        assert!(mapper.get(&dir).is_none());
    }

    #[test]
    fn link_and_exchange_keep_each_inode_links_consistent() {
        let mut mapper = InodeMapper::new(());
        let a = mapper
            .insert_child(&ROOT_INODE, "a".into(), |_| ())
            .unwrap();
        let c = mapper
            .insert_child(&ROOT_INODE, "c".into(), |_| ())
            .unwrap();
        mapper.link(&a, &ROOT_INODE, "b".into()).unwrap();
        assert_eq!(mapper.get(&a).unwrap().links.len(), 2);
        mapper
            .exchange(&ROOT_INODE, OsStr::new("b"), &ROOT_INODE, OsStr::new("c"))
            .unwrap();
        assert_eq!(
            mapper.lookup(&ROOT_INODE, OsStr::new("b")).unwrap().inode,
            &c
        );
        assert_eq!(
            mapper.lookup(&ROOT_INODE, OsStr::new("c")).unwrap().inode,
            &a
        );
        let paths = mapper.resolve(&a).unwrap();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&vec![OsString::from("a")]));
        assert!(paths.contains(&vec![OsString::from("c")]));
    }
}
