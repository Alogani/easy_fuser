use std::cmp::max;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::io;

/// Trait to delegate filesystem operations
pub trait FilesystemBackend {
    fn readdir(&self, parent_path: &OsStr) -> Result<Vec<OsString>, io::Error>;
    fn rename(&self, path: &OsStr, new_path: &OsStr) -> Result<(), io::Error>;
    fn unlink(&self, path: &OsStr) -> Result<(), io::Error>;
}

/// Struct representing the in-memory inode-to-path mapping.
///
/// The paths are cached, so change from the backend will lead to errors.
///
/// Paths are retrieved and constructed lazily,
/// so it might not be efficient for deeply nested structure,
/// however it has a minimum memory and cpu overhead in most conditions
pub struct InodePathMapper {
    // Map of inode -> (path, children)
    inodes: HashMap<u64, Inode>,
    // Counter for assigning new inodes
    next_inode: u64,
    // Backend for filesystem operations
    backend: Box<dyn FilesystemBackend>,
}

struct Inode {
    name_ptr: *const OsStr,
    parent: u64,
    children: Option<HashMap<OsString, u64>>,
}

const ROOT_INODE: u64 = 1;

impl InodePathMapper {
    /// Create a new InodePathMapper with the root inode (1) initialized
    pub fn new(backend: Box<dyn FilesystemBackend>) -> Self {
        let mut inodes = HashMap::new();
        let root: &'static str = "";

        inodes.insert(
            ROOT_INODE,
            Inode {
                name_ptr: root as *const str as *const OsStr,
                parent: ROOT_INODE,
                children: None,
            },
        );

        Self {
            inodes,
            next_inode: 2, // Start assigning inodes from 2
            backend,
        }
    }

    pub fn lookup(&mut self, parent_inode: u64, name: &OsStr) -> Result<u64, io::Error> {
        let children = self.get_children(parent_inode)?;

        children
            .get(name)
            .copied()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Child not found"))
    }

    pub fn readdir(&mut self, parent_inode: u64) -> Result<Vec<u64>, io::Error> {
        let children = self.get_children(parent_inode)?;
        Ok(children.values().map(|v| v.clone()).collect())
    }

    fn get_children(&mut self, parent_inode: u64) -> Result<&HashMap<OsString, u64>, io::Error> {
        if self
            .inodes
            .get(&parent_inode)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Parent inode not found"))?
            .children
            .is_none()
        {
            let parent_path = self.get_path(parent_inode)?;

            let child_list = self.backend.readdir(&parent_path)?;

            // Step 4: Create the children map
            let mut children_map = HashMap::new();
            for child_name in child_list {
                let new_inode = self.next_inode;
                let name_ptr = child_name.as_os_str() as *const OsStr;
                self.next_inode += 1;

                children_map.insert(child_name, new_inode);

                // Add the child inode to the map
                self.inodes.insert(
                    new_inode,
                    Inode {
                        name_ptr,
                        parent: parent_inode,
                        children: None,
                    },
                );
            }

            // Step 5: Update the parent's children field
            let parent = self
                .inodes
                .get_mut(&parent_inode)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Parent inode not found"))?;
            parent.children = Some(children_map);
        }

        Ok(self
            .inodes
            .get(&parent_inode)
            .unwrap()
            .children
            .as_ref()
            .unwrap())
    }

    pub fn rename(
        &mut self,
        parent: u64,
        name: &OsStr,
        new_parent: u64,
        new_name: OsString,
    ) -> Result<(), io::Error> {
        let inode = self.lookup(parent, name)?;
        let new_path = {
            let mut path = self.get_path(new_parent)?;
            path.push(OsString::from("/"));
            path.push(&new_name);
            path
        };

        self.backend.rename(&self.get_path(inode)?, &new_path)?;

        let new_name_ptr: *const OsStr = new_name.as_os_str();
        let _ = self
            .inodes
            .get_mut(&parent)
            .unwrap()
            .children
            .as_mut()
            .unwrap()
            .remove(name);
        self.inodes
            .get_mut(&new_parent)
            .unwrap()
            .children
            .as_mut()
            .unwrap()
            .insert(new_name, inode);
        self.inodes.get_mut(&inode).unwrap().name_ptr = new_name_ptr;

        Ok(())
    }

    pub fn unlink(&mut self, parent: u64, name: &OsStr) -> Result<(), io::Error> {
        let inode = self.lookup(parent, name)?;

        self.backend.unlink(&self.get_path(inode)?)?;

        let _ = self
            .inodes
            .get_mut(&parent)
            .unwrap()
            .children
            .as_mut()
            .unwrap()
            .remove(name);
        let _ = self.inodes.remove(&inode);

        Ok(())
    }

    pub fn get_path(&self, inode: u64) -> Result<OsString, io::Error> {
        let mut result = OsString::new();
        let mut inode = inode;
        loop {
            let (parent, name) = {
                let Inode {
                    name_ptr,
                    parent,
                    children: _,
                } = self.inodes.get(&inode).ok_or(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Parent inode not found",
                ))?;
                (*parent, unsafe { &**name_ptr })
            };
            let old_result = result;
            let required_capacity = max(old_result.capacity(), name.len() + old_result.len() + 1);
            result = OsString::with_capacity(required_capacity);
            result.push(OsStr::new("/"));
            result.push(name);
            result.push(&old_result);
            inode = parent;
            if inode == ROOT_INODE {
                break;
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::ffi::{OsStr, OsString};
    use std::io;

    struct MockFilesystemBackend {
        files: RefCell<HashSet<String>>, // Store full paths as a simple set
    }

    impl MockFilesystemBackend {
        fn new(initial_files: Vec<String>) -> Self {
            Self {
                files: RefCell::new(initial_files.into_iter().collect()),
            }
        }
    }

    impl FilesystemBackend for MockFilesystemBackend {
        fn readdir(&self, parent_path: &OsStr) -> Result<Vec<OsString>, io::Error> {
            let parent_path_str = parent_path.to_string_lossy();
            let parent_path_with_slash = if parent_path_str.ends_with('/') {
                parent_path_str.to_string()
            } else {
                format!("{}/", parent_path_str)
            };

            let files = self.files.borrow();
            let children = files
                .iter()
                .filter_map(|path| {
                    if path.starts_with(&parent_path_with_slash) {
                        let relative_path = path.trim_start_matches(&parent_path_with_slash);
                        relative_path
                            .split('/')
                            .next()
                            .map(|child| child.to_string())
                    } else {
                        None
                    }
                })
                .collect::<HashSet<_>>();

            // Convert to OsString and return sorted
            let mut result = children.into_iter().map(OsString::from).collect::<Vec<_>>();
            result.sort();
            Ok(result)
        }

        fn rename(&self, path: &OsStr, new_path: &OsStr) -> Result<(), io::Error> {
            let mut files = self.files.borrow_mut();
            let old_path = path.to_string_lossy().to_string();
            let new_path = new_path.to_string_lossy().to_string();

            if files.remove(&old_path) {
                files.insert(new_path);
                Ok(())
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "Path not found"))
            }
        }

        fn unlink(&self, path: &OsStr) -> Result<(), io::Error> {
            let mut files = self.files.borrow_mut();
            let path = path.to_string_lossy().to_string();
            if files.remove(&path) {
                Ok(())
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "Path not found"))
            }
        }
    }

    #[test]
    fn test_lookup() {
        let mock_backend =
            MockFilesystemBackend::new(vec!["/folder".to_string(), "/folder/subfile".to_string()]);
        let mut mapper = InodePathMapper::new(Box::new(mock_backend));

        let parent_inode = 1; // root inode
        let folder_inode_res = mapper.lookup(parent_inode, OsStr::new("folder"));

        assert!(folder_inode_res.is_ok());
        let folder_inode = folder_inode_res.unwrap();
        assert_eq!(
            mapper.get_path(folder_inode).unwrap(),
            OsString::from("/folder")
        );

        let subfile_inode = mapper.lookup(folder_inode, OsStr::new("subfile"));

        assert!(subfile_inode.is_ok());
        assert_eq!(
            mapper.get_path(subfile_inode.unwrap()).unwrap(),
            OsString::from("/folder/subfile")
        );
    }

    #[test]
    fn test_readdir() {
        let mock_backend = MockFilesystemBackend::new(vec![
            "/file".to_string(),
            "/folder".to_string(),
            "/folder/subfile".to_string(),
        ]);
        let mut mapper = InodePathMapper::new(Box::new(mock_backend));

        let parent_inode = 1; // root inode
        let children_inodes = mapper.readdir(parent_inode).unwrap();

        assert_eq!(children_inodes.len(), 2);

        let paths: Vec<_> = children_inodes
            .iter()
            .map(|&inode| mapper.get_path(inode).unwrap())
            .collect();
        assert!(paths.contains(&OsString::from("/file")));
        assert!(paths.contains(&OsString::from("/folder")));

        let folder_inode = mapper.lookup(parent_inode, OsStr::new("folder")).unwrap();
        let children_inodes = mapper.readdir(folder_inode).unwrap();
        assert_eq!(children_inodes.len(), 1);
        let paths: Vec<_> = children_inodes
            .iter()
            .map(|&inode| mapper.get_path(inode).unwrap())
            .collect();
        assert_eq!(paths[0], OsString::from("/folder/subfile"));
    }

    #[test]
    fn test_rename_in_folder() {
        let mock_backend =
            MockFilesystemBackend::new(vec!["/folder".to_string(), "/folder/file1".to_string()]);
        let mut mapper = InodePathMapper::new(Box::new(mock_backend));

        let root_inode = 1; // root inode
        let folder_inode = mapper.lookup(root_inode, OsStr::new("folder")).unwrap();
        let file_inode = mapper.lookup(folder_inode, OsStr::new("file1")).unwrap();

        // Rename the file inside the folder
        mapper
            .rename(
                folder_inode,
                OsStr::new("file1"),
                folder_inode,
                OsString::from("file_renamed"),
            )
            .unwrap();

        // Verify that the file has been renamed
        let new_file_path = mapper.get_path(file_inode).unwrap();
        assert_eq!(new_file_path, OsString::from("/folder/file_renamed"));
    }

    #[test]
    fn test_remove_subfolder_and_verify_subfile_inexistence() {
        // Initial setup with mock backend
        let mock_backend = MockFilesystemBackend::new(vec![
            "/folder".to_string(),
            "/folder/subfolder".to_string(),
            "/folder/subfolder/subfile".to_string(),
        ]);
        let mut mapper = InodePathMapper::new(Box::new(mock_backend));

        // Simulate looking up the folder inode
        let root_inode = 1; // root inode
        let folder_inode = mapper.lookup(root_inode, OsStr::new("folder"));
        assert!(folder_inode.is_ok());
        let folder_inode = folder_inode.unwrap();

        // Simulate looking up the subfolder inode
        let subfolder_inode = mapper.lookup(folder_inode, OsStr::new("subfolder"));
        assert!(subfolder_inode.is_ok());
        let subfolder_inode = subfolder_inode.unwrap();

        // Simulate looking up the subfile inside the subfolder
        let subfile_inode = mapper.lookup(subfolder_inode, OsStr::new("subfile"));
        assert!(subfile_inode.is_ok());

        // Remove the subfolder
        mapper
            .unlink(folder_inode, OsStr::new("subfolder"))
            .unwrap();

        // Verify the subfolder is no longer found
        let subfolder_lookup_result = mapper.lookup(folder_inode, OsStr::new("subfolder"));
        assert!(subfolder_lookup_result.is_err());

        // Verify the subfile inside the subfolder is also not found
        let subfile_lookup_result = mapper.lookup(subfolder_inode, OsStr::new("subfile"));
        assert!(subfile_lookup_result.is_err());
    }
}
