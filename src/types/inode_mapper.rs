use std::borrow::Borrow;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::hash::Hash;
use std::sync::Arc;

use super::{Inode, ROOT_INODE};

/// Core structure for managing inodes and their relationships.
///
/// InodeMapper<T> efficiently stores and manages inodes, their associated data,
/// and parent-child relationships. It uses InodeData<T> for internal storage,
/// with InodeValue<T> representing individual inode entries.
///
/// # Note
/// - T is the type of data associated with each inode.
/// - Uses OsStringWrapper for efficient string storage and comparison.
/// - Maintains a next_inode counter for generating unique inode values.
pub struct InodeMapper<T> {
    data: InodeData<T>,
    root_inode: Inode,
    next_inode: Inode,
}

struct InodeData<T> {
    inodes: HashMap<Inode, InodeValue<T>>,
    children: HashMap<Inode, HashMap<OsStringWrapper, Inode>>,
}

#[derive(Debug)]
struct InodeValue<T> {
    parent: Inode,
    name: OsStringWrapper,
    data: T,
}

pub struct ValueCreatorParams<'a, T> {
    pub new_inode: &'a Inode,
    pub parent_inode: &'a Inode,
    pub child_name: &'a OsStr,
    pub existing_data: Option<&'a T>,
}

trait ValueCreator<'a, T> {
    fn create(&self, params: ValueCreatorParams<'a, T>) -> T;
}

pub struct LookupResult<'a, T> {
    pub inode: &'a Inode,
    pub name: &'a Arc<OsString>,
    pub data: &'a T,
}

pub struct InodeInfo<'a, T> {
    pub parent_inode: &'a Inode,
    pub name: &'a Arc<OsString>,
    pub data: &'a T,
}

pub struct InodeInfoMut<'a, T> {
    pub parent_inode: &'a Inode,
    pub name: &'a mut Arc<OsString>,
    pub data: &'a mut T,
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

/// A wrapper around `Arc<OsString>` for efficient storage and comparison in hash maps.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct OsStringWrapper(Arc<OsString>);

impl AsRef<Arc<OsString>> for OsStringWrapper {
    fn as_ref(&self) -> &Arc<OsString> {
        &self.0
    }
}

impl AsMut<Arc<OsString>> for OsStringWrapper {
    fn as_mut(&mut self) -> &mut Arc<OsString> {
        &mut self.0
    }
}

impl Borrow<OsStr> for OsStringWrapper {
    fn borrow(&self) -> &OsStr {
        self.0.as_os_str()
    }
}

impl<T: Send + Sync + 'static> InodeMapper<T> {
    /// Creates a new `InodeMapper` instance with the root inode initialized.
    ///
    /// This function initializes the `InodeMapper` with an empty structure and sets up the root inode
    /// with the provided data. The root inode is assigned an empty name and its parent is set to itself.
    pub fn new(data: T) -> Self {
        let mut result = InodeMapper {
            data: InodeData {
                inodes: HashMap::new(),
                children: HashMap::new(),
            },
            root_inode: ROOT_INODE.clone(),
            next_inode: ROOT_INODE.add_one(),
        };
        result.data.inodes.insert(
            ROOT_INODE.clone(),
            InodeValue {
                parent: ROOT_INODE.clone(),
                name: OsStringWrapper(Arc::new(OsString::from(""))),
                data,
            },
        );
        result
    }

    pub fn get_root_inode(&self) -> Inode {
        self.root_inode.clone()
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
        F: Fn(ValueCreatorParams<'_, T>) -> T,
    {
        // Wrap `child` in `OsStringWrapper` for efficient storage and comparison
        let child = OsStringWrapper(Arc::new(child));

        let mut is_new = false;
        let inode = self
            .data
            .children
            .entry(parent.clone())
            .or_insert_with(HashMap::new)
            .entry(child.clone())
            .or_insert_with(|| {
                is_new = true;
                self.next_inode.clone()
            }).clone();
        if is_new {
            self.next_inode = inode.add_one();
            self.data
                .inodes
                .insert(
                    inode.clone(),
                    InodeValue {
                        parent: parent.clone(),
                        name: child.clone(),
                        data: value_creator(ValueCreatorParams{
                            parent_inode: &parent,
                            new_inode: &inode,
                            child_name: &child.as_ref(),
                            existing_data: None
                        }),
                    },
                );
        } else {
            let inode_value = &mut self.data.inodes.get_mut(&inode).unwrap();
            inode_value.data = value_creator(ValueCreatorParams{
                parent_inode: &parent,
                new_inode: &inode,
                child_name: &child.as_ref(),
                existing_data: Some(&inode_value.data),
            });
        }
        return inode;
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
        F: Fn(ValueCreatorParams<'_, T>) -> T,
    {
        if self.data.inodes.get(parent).is_none() {
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
    /// - Returns Err(InsertError::ParentNotFound) if the parent doesn't exist.
    /// - If successful, returns Ok(Vec<Inode>) containing the newly created or existing child inodes.
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
        F: Fn(ValueCreatorParams<'_, T>) -> T,
    {
        if self.data.inodes.get(parent).is_none() {
            return Err(InsertError::ParentNotFound);
        }

        // Reserve space in the parent's children HashMap
        if let Some(parent_children) = self.data.children.get_mut(&parent) {
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
    pub fn batch_insert<F, G>(
        &mut self,
        parent: &Inode,
        entries: Vec<(Vec<OsString>, F)>,
        default_parent_creator: G
    ) -> Result<(), InsertError>
    where
        F: Fn(ValueCreatorParams<'_, T>) -> T,
        G: Fn(ValueCreatorParams<'_, T>) -> T
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
            let parent_inode = self.ensure_path_exists(&mut path_cache, &path, &default_parent_creator);
            self.insert_child_unchecked(&parent_inode, name, value_creator);
        }
    
        Ok(())
    }

    fn ensure_path_exists(
        &mut self,
        path_cache: &mut HashMap<Vec<OsString>, Inode>,
        path: &[OsString],
        default_parent_creator: &impl Fn(ValueCreatorParams<T>) -> T
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
                        self.insert_child_unchecked(&current_inode, component.clone(), |mut value_creator_params| {
                            value_creator_params.existing_data = None;
                            default_parent_creator(value_creator_params)}
                        )
                    }
                } else {
                    self.insert_child_unchecked(&current_inode, component.clone(), |mut value_creator_params| {
                        value_creator_params.existing_data = None;
                        default_parent_creator(value_creator_params)}
                    )
                };
                path_cache.insert(current_path.to_vec(), new_inode.clone());
                current_inode = new_inode;
            }
        }
        current_inode
    }

    /// Resolves an inode to its full path components.
    ///
    /// This method traverses from the given inode up to the root, collecting all parent names along the way.
    /// The resulting path is in reverse order (from leaf to root).
    ///
    /// # Notes
    /// - Returns `None` if any inode in the path is not found, indicating an incomplete or invalid path.
    /// - The root inode is identified when its parent is equal to itself.
    /// - The returned `Vec` contains `OsStringWrapper`s, which are efficient wrappers around `OsString`s.
    pub fn resolve(&self, inode: &Inode) -> Option<Vec<&OsStr>> {
        let mut result: Vec<&OsStr> = Vec::new();
        let mut current_inode = inode;

        loop {
            if let Some(inode_value) = self.data.inodes.get(current_inode) {
                // If we've reached the root (parent is itself), break the loop without including it
                if inode_value.parent == *current_inode {
                    break;
                }

                result.push(inode_value.name.as_ref());

                // Move to the parent
                current_inode = &inode_value.parent;
            } else {
                // If we can't find the inode, the path is incomplete, so return None
                return None;
            }
        }

        Some(result)
    }

    pub fn get(&self, inode: &Inode) -> Option<InodeInfo<T>> {
        self.data.inodes.get(inode).map(|inode_value| {
            InodeInfo {
                parent_inode: &inode_value.parent,
                name: inode_value.name.as_ref(),
                data: &inode_value.data,
            }
        })
    }

    pub fn get_mut(&mut self, inode: &Inode) -> Option<InodeInfoMut<T>> {
        self.data.inodes.get_mut(inode).map(|inode_value| {
            InodeInfoMut {
                parent_inode: &inode_value.parent,
                name: inode_value.name.as_mut(),
                data: &mut inode_value.data,
            }
        })
    }

    // Retrieves all children of a given parent inode.
    ///
    /// # Note
    /// - Does not check if the parent inode exists.
    /// - Returns an empty vector if the parent has no children or doesn't exist.
    pub fn get_children(&self, parent: &Inode) -> Vec<(&Arc<OsString>, &Inode)> {
        self.data.children
            .get(parent)
            .map(|children| {
                children.iter()
                    .map(|(name, inode)| (name.as_ref(), inode))
                    .collect()
            })
            .unwrap_or(vec![])
    }

    /// Looks up a child inode by its parent inode and name
    pub fn lookup(&self, parent: &Inode, name: &OsStr) -> Option<LookupResult<T>> {
        self.data
            .children
            .get(parent)
            .and_then(|children| children.get(name))
            .map(|child_inode| {
                let inode_value = self.data.inodes.get(child_inode).unwrap();
                LookupResult {
                    inode: child_inode,
                    name: inode_value.name.as_ref(),
                    data: &inode_value.data,
                }
            })
    }

    /// Renames a child inode from one parent to another
    pub fn rename(
        &mut self,
        parent: &Inode,
        oldname: &OsStr,
        newparent: &Inode,
        newname: OsString,
    ) -> Result<Option<(Inode, T)>, RenameError> {
        let newname = OsStringWrapper(Arc::new(newname));

        // Check if the new parent exists
        if !self.data.inodes.contains_key(parent) {
            return Err(RenameError::ParentNotFound);
        }
        if !self.data.inodes.contains_key(newparent) {
            return Err(RenameError::NewParentNotFound);
        }

        // Remove the child from the old parent
        let mut is_parent_empty = false;
        let child_inode = self
            .data
            .children
            .get_mut(parent)
            .ok_or(RenameError::NotFound)
            .and_then(|parent_children| {
                let child_inode = parent_children
                    .remove(oldname)
                    .ok_or(RenameError::NotFound)?;
                if parent_children.is_empty() {
                    is_parent_empty = true;
                }
                Ok(child_inode)
            })?;

        // Remove the old parent if it's now empty
        if is_parent_empty {
            self.data.children.remove(parent);
        }

        // Update the inode value with the new parent and name
        self.data.inodes.get_mut(&child_inode).map(|inode_value| {
            inode_value.parent = newparent.clone();
            inode_value.name = newname.clone();
        });

        // Insert the child into the new parent's children map
        if let Some(old_inode) = self
            .data
            .children
            .get_mut(newparent)
            .and_then(|children| children.insert(newname, child_inode))
        {
            let InodeValue {
                parent: _,
                name: _,
                data,
            } = self.data.inodes.remove(&old_inode).unwrap();
            Ok(Some((old_inode, data)))
        } else {
            Ok(None)
        }
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
    pub fn remove(&mut self, inode: &Inode) -> Option<T> {
        #![cfg(debug_assertions)]
        if *inode == ROOT_INODE {
            panic!("Cannot remove ROOT");
        }
        if let Some(inode_value) = self.data.inodes.remove(inode) {
            // Remove this inode from its parent's children
            if let Some(parent_children) = self.data.children.get_mut(&inode_value.parent) {
                parent_children.remove(&inode_value.name);

                // If the parent's children map is now empty, remove it from the children HashMap
                if parent_children.is_empty() {
                    self.data.children.remove(&inode_value.parent);
                }
            }

            // Cascade remove all children
            if let Some(children) = self.data.children.remove(inode) {
                for child_inode in children.values() {
                    self.remove(child_inode);
                }
            }
            Some(inode_value.data)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashSet;
    use std::ffi::OsString;

    use crate::types::inode_mapper::InodeMapper;
    use crate::types::Inode;
    use crate::ROOT_INODE;

    #[test]
    fn test_insert_child_returns_old_inode() {
        let mut mapper = InodeMapper::new(0);
        let root = mapper.get_root_inode();
        let child_name = OsString::from("child");
    
        // Insert the first child
        let first_child_inode = Inode::from(2);
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
            entries.push((path, move |_| i));
            expected_inodes.insert(Inode::from(i + 2)); // Start from 2 to avoid conflict with root_inode
        }
    
        // Perform batch insert
        let result = mapper.batch_insert(&ROOT_INODE, entries, move |params: ValueCreatorParams<'_, u64>| 0);
    
        // Verify results
        assert!(result.is_ok(), "Batch insert should succeed");
    
        // Check if all inserted inodes exist
        for i in 2..=(FILE_COUNT as u64 + 1) {
            let inode = Inode::from(i);
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
                        *lookup_result_unwraped.data,
                        i as u64,
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
    
        let dir_inode = mapper.insert_child(&mapper.get_root_inode(), OsString::from("dir"), |_| ()).unwrap();
        let file_inode =mapper.insert_child(&dir_inode, OsString::from("file.txt"), |_| ()).unwrap();
    
        // Resolve the file inode
        let path = mapper.resolve(&file_inode).unwrap();
    
        // Check the resolved path (it should be in reverse order)
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], "file.txt");
        assert_eq!(path[1], "dir");
    
        // Resolve the root inode (should be empty)
        let root_path = mapper.resolve(&ROOT_INODE).unwrap();
        assert!(root_path.is_empty());
    
        // Try to resolve a non-existent inode
        assert!(mapper.resolve(&Inode::from(999)).is_none());
    }

    #[test]
    fn test_resolve_invalid_inode() {
        let mapper = InodeMapper::new( 0);
        let invalid_inode = Inode::from(999);

        // Attempt to resolve an invalid inode
        let result = mapper.resolve(&invalid_inode);

        // Assert that the result is None
        assert!(
            result.is_none(),
            "Resolving an invalid inode should return None"
        );
    }

    #[test]
    fn test_rename_child_inode() {
        let mut mapper = InodeMapper::new(());
        let root = mapper.get_root_inode();
    
        // Insert initial structure
        let parent1 = mapper.insert_child(&root, OsString::from("parent1"), |_| ()).unwrap();
        let parent2 = mapper.insert_child(&root, OsString::from("parent2"), |_| ()).unwrap();
        let child = mapper.insert_child(&parent1, OsString::from("old_name"), |_| ()).unwrap();
        mapper.insert_child(&parent2, OsString::from("dummy"), |_| ()).unwrap();

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
        let inode_value = mapper.get(&child).unwrap();
        assert_eq!(inode_value.parent_inode, &parent2);
        assert_eq!(inode_value.name.as_os_str(), OsStr::new("new_name"));
    }

    #[test]
    fn test_rename_non_existent_child() {
        let mut mapper = InodeMapper::new(0);
    
        // Insert parent inodes
        let root = mapper.get_root_inode();
        let parent = mapper.insert_child(&root, OsString::from("parent"), |_| 1).unwrap();
        let newparent = mapper.insert_child(&root, OsString::from("newparent"), |_| 2).unwrap();
    
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
        let child1 = Inode::from(2);
        let child2 = Inode::from(3);
        let grandchild1 = Inode::from(4);
        let grandchild2 = Inode::from(5);
        let great_grandchild = Inode::from(6);
    
        // Create a deeper nested structure
        mapper.insert_child(&ROOT_INODE, OsString::from("child1"), |_| ()).unwrap();
        mapper.insert_child(&ROOT_INODE, OsString::from("child2"), |_| ()).unwrap();
        mapper.insert_child(&child1, OsString::from("grandchild1"), |_| ()).unwrap();
        mapper.insert_child(&child1, OsString::from("grandchild2"), |_| ()).unwrap();
        mapper.insert_child(&grandchild1, OsString::from("great_grandchild"), |_| ()).unwrap();
    
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
}
