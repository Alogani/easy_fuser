use std::path::{Path, PathBuf};

use types::{FuseDirEntry, PosixError};

use super::fd_bridge::FileDescriptorBridge;
use crate::types::*;
use crate::*;

pub struct PassthroughFs {
    _repo: PathBuf,
}

impl PassthroughFs {
    pub fn new(repo: &Path) -> Self {
        Self { _repo: repo.into() }
    }
}

impl FileDescriptorBridge for PassthroughFs {}

impl FuseAPI for PassthroughFs {
    fn getattr(
        &mut self,
        _req: types::RequestInfo,
        ino: u64,
        file_handle: Option<types::FileHandle>,
    ) -> Result<types::AttributeResponse, std::io::Error> {
        if ino == 1 {
            posix_fs::lookup(&self._repo, Some(ino))
        } else {
            Err(PosixError::FUNCTION_NOT_IMPLEMENTED.into())
        }
    }

    fn lookup(
        &mut self,
        _req: types::RequestInfo,
        parent_inode: u64,
        name: &std::ffi::OsStr,
    ) -> Result<types::AttributeResponse, std::io::Error> {
        posix_fs::lookup(&self._repo.join("file"), Some(parent_inode + 1))
    }

    fn readdir(
        &mut self,
        _req: types::RequestInfo,
        ino: u64,
        file_handle: types::FileHandle,
    ) -> Result<Vec<types::FuseDirEntry>, std::io::Error> {
        static mut i: u64 = 1;
        eprintln!("REPO={:?}", &self._repo);
        let mut response_entries = Vec::from([
            FuseDirEntry {
                inode: ino,
                name: ".".into(),
                kind: FileType::Directory,
            },
            FuseDirEntry {
                inode: unsafe {
                    i += 1;
                    i
                },
                name: "..".into(),
                kind: FileType::Directory,
            },
        ]);
        response_entries.extend(posix_fs::readdir(&self._repo)?.iter().map(|(name, t)| {
            FuseDirEntry {
                inode: unsafe {
                    i += 1;
                    i
                },
                name: name.clone(),
                kind: *t,
            }
        }));
        eprintln!("ENTRIES={:?}", response_entries);
        Ok(response_entries)
    }
}
