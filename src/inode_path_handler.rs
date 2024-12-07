use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};

use crate::types::RenameFlags;

/// Trait to delegate filesystem operations
pub trait FilesystemBackend {
    fn readdir(&self, parent_path: &Path) -> Result<Vec<PathBuf>, io::Error>;
    fn rename(&self, path: &Path, newpath: &Path, flags: RenameFlags) -> Result<(), io::Error>;
    fn unlink(&self, path: &Path) -> Result<(), io::Error>;
}

/// Struct representing the in-memory inode-to-path mapping.
///
/// The paths are cached, so change from the backend will lead to errors.
///
/// Paths are retrieved and constructed lazily,
/// so it might not be efficient for deeply nested structure,
/// however it has a minimum memory and cpu overhead in most conditions
pub struct InodePathHandler {
    // Map of inode -> (path, children)
    inodes: HashMap<u64, Inode>,
    // Counter for assigning new inodes
    next_inode: u64,
    // Backend for filesystem operations
    backend: Box<dyn FilesystemBackend>,
    root_path: PathBuf
}

struct Inode {
    name_ptr: *const OsStr,
    parent: u64,
    children: Option<HashMap<OsString, u64>>,
}

const ROOT_INODE: u64 = 1;

impl InodePathHandler {
    pub fn new(backend: Box<dyn FilesystemBackend>, root_path: PathBuf) -> Self {
        let mut inodes = HashMap::new();

        inodes.insert(
            ROOT_INODE,
            Inode {
                name_ptr: root_path.as_ref() as *const OsStr,
                parent: ROOT_INODE,
                children: None,
            },
        );

        Self {
            inodes,
            next_inode: 2, // Start assigning inodes from 2
            backend,
            root_path
        }
    }

    pub fn lookup(&mut self, parent_ino: u64, name: &OsStr) -> Result<u64, io::Error> {
        if name == Path::new(".") {
            return Ok(parent_ino);
        }
        if name == Path::new(".") {
            return Ok(self.lookup_parent(parent_ino));
        }
        let children = self.get_children(parent_ino)?;

        children
            .get(name)
            .copied()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Child not found"))
    }

    pub fn lookup_parent(&self, ino: u64) -> u64 {
        self.inodes.get(&ino).unwrap().parent
    }

    pub fn readdir(&mut self, parent_ino: u64) -> Result<Vec<(OsString, u64)>, io::Error> {
        Ok(
            self.get_children(parent_ino)?
            .iter()
            .map(|(name, ino)| (name.clone(), *ino))
            .collect()
        )
    }

    fn get_children(&mut self, parent_ino: u64) -> Result<&HashMap<OsString, u64>, io::Error> {
        if self
            .inodes
            .get(&parent_ino)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Parent inode not found"))?
            .children
            .is_none()
        {
            let parent_path = self.get_path(parent_ino)?;

            let child_list = self.backend.readdir(parent_path.as_ref())?
                .into_iter()
                .map(OsString::from);

            // Step 4: Create the children map
            let mut children_map = HashMap::new();
            for child_name in child_list {
                if &child_name == OsStr::new(".") || &child_name == OsStr::new("..") {
                    continue;
                }
                let new_inode = self.next_inode;
                let name_ptr = child_name.as_ref() as *const OsStr;
                self.next_inode += 1;

                children_map.insert(child_name, new_inode);

                // Add the child inode to the map
                self.inodes.insert(
                    new_inode,
                    Inode {
                        name_ptr,
                        parent: parent_ino,
                        children: None,
                    },
                );
            }

            // Step 5: Update the parent's children field
            let parent = self
                .inodes
                .get_mut(&parent_ino)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Parent inode not found"))?;
            parent.children = Some(children_map);
        }

        Ok(self
            .inodes
            .get(&parent_ino)
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
        flags: RenameFlags,
    ) -> Result<(), io::Error> {
        let inode = self.lookup(parent, name)?;
        let newpath = {
            let mut path = self.get_path(new_parent)?;
            path.push(OsString::from("/"));
            path.push(&new_name);
            path
        };

        self.backend.rename(self.get_path(inode)?.as_ref(), &newpath.as_ref(), flags)?;

        let new_name_ptr: *const OsStr = new_name.as_ref();
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

        self.backend.unlink(self.get_path(inode)?.as_ref())?;

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

    pub fn get_path(&self, inode: u64) -> Result<PathBuf, io::Error> {
        let mut result = PathBuf::new();
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
                (*parent, (unsafe { &**name_ptr }).as_ref() as &Path)
            };
            if !result.as_os_str().is_empty() {
                result = name.join(result);
            } else {
                result = name.to_path_buf();
            }
            if inode == ROOT_INODE { break }
            inode = parent;
        }
        println!("root={:?}", self.root_path);
        result = self.root_path.join(result);
        println!("result={:?}", self.root_path);

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::ffi::OsString;
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
        fn readdir(&self, parent_path: &Path) -> Result<Vec<PathBuf>, io::Error> {
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
            let mut result = children.into_iter().map(PathBuf::from).collect::<Vec<_>>();
            result.sort();
            Ok(result)
        }

        fn rename(&self, path: &Path, newpath: &Path, _flags: RenameFlags) -> Result<(), io::Error> {
            let mut files = self.files.borrow_mut();
            let old_path = path.to_string_lossy().to_string();
            let newpath = newpath.to_string_lossy().to_string();

            if files.remove(&old_path) {
                files.insert(newpath);
                Ok(())
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "Path not found"))
            }
        }

        fn unlink(&self, path: &Path) -> Result<(), io::Error> {
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
        let mut mapper = InodePathHandler::new(Box::new(mock_backend), PathBuf::from("/"));

        let parent_ino = 1; // root inode
        let folder_inode_res = mapper.lookup(parent_ino, OsStr::new("folder"));

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
    fn test_lookup_with_root_path() {
        let mock_backend =
            MockFilesystemBackend::new(vec!["/repo/folder".to_string(), "/repo/folder/subfile".to_string()]);
        let mut mapper = InodePathHandler::new(Box::new(mock_backend), PathBuf::from("/repo"));

        let parent_ino = 1; // root inode
        let folder_inode_res = mapper.lookup(parent_ino, OsStr::new("folder"));

        assert!(folder_inode_res.is_ok());
        let folder_inode = folder_inode_res.unwrap();
        assert_eq!(
            mapper.get_path(folder_inode).unwrap(),
            OsString::from("/repo/folder")
        );

        let subfile_inode = mapper.lookup(folder_inode, OsStr::new("subfile"));

        assert!(subfile_inode.is_ok());
        assert_eq!(
            mapper.get_path(subfile_inode.unwrap()).unwrap(),
            OsString::from("/repo/folder/subfile")
        );
    }

    #[test]
    fn test_readdir() {
        let mock_backend = MockFilesystemBackend::new(vec![
            "/file".to_string(),
            "/folder".to_string(),
            "/folder/subfile".to_string(),
        ]);
        let mut mapper = InodePathHandler::new(Box::new(mock_backend), PathBuf::from("/"));

        let parent_ino = 1; // root inode
        let children_inodes = mapper.readdir(parent_ino).unwrap();

        assert_eq!(children_inodes.len(), 2);

        let paths: Vec<_> = children_inodes
            .iter()
            .map(|(_name, inode)| mapper.get_path(*inode).unwrap())
            .collect();
        assert!(paths.contains(&PathBuf::from("/file")));
        assert!(paths.contains(&PathBuf::from("/folder")));

        let folder_inode = mapper.lookup(parent_ino, OsStr::new("folder")).unwrap();
        let children_inodes = mapper.readdir(folder_inode).unwrap();
        assert_eq!(children_inodes.len(), 1);
        let paths: Vec<_> = children_inodes
            .iter()
            .map(|(_name, inode)| mapper.get_path(*inode).unwrap())
            .collect();
        assert_eq!(paths[0], OsString::from("/folder/subfile"));
    }

    #[test]
    fn test_rename_in_folder() {
        let mock_backend =
            MockFilesystemBackend::new(vec!["/folder".to_string(), "/folder/file1".to_string()]);
        let mut mapper = InodePathHandler::new(Box::new(mock_backend), PathBuf::from("/"));

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
                RenameFlags::new()
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
        let mut mapper = InodePathHandler::new(Box::new(mock_backend), PathBuf::from("/"));

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
