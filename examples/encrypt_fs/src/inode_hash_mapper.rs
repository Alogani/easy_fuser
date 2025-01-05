use easy_fuser::{inode_mapper::*, prelude::*};

use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

/// We could also more simply use easy_fuser mapper and compute each time the value of encrypted path
///
/// The implementation is very similar to easy_fuser::core::inode_mapping::ComponentsResolver
/// to manage also nlookup count
pub struct InodeHashMapper {
    mapper: InodeMapper<InodeData>,
}

struct InodeData {
    hashed_name: String,
    nlookup: AtomicU64,
}

impl InodeHashMapper {
    pub fn new() -> Self {
        InodeHashMapper {
            mapper: InodeMapper::new(InodeData {
                hashed_name: String::new(),
                nlookup: AtomicU64::new(0),
            }),
        }
    }

    pub fn resolve_hashed_path(&self, inode: &Inode) -> Option<PathBuf> {
        Some(
            self.mapper
                .resolve(inode)?
                .into_iter()
                .map(|inode_info| inode_info.data.hashed_name.clone())
                .collect::<PathBuf>(),
        )
    }

    pub fn lookup(&self, parent: &Inode, child: &OsStr, increment: bool) -> Option<Inode> {
        let LookupResult {
            inode,
            name: _,
            data,
        } = self.mapper.lookup(parent, child)?;
        if increment {
            data.nlookup.fetch_add(1, Ordering::SeqCst);
        };
        Some(inode.clone())
    }

    pub fn add_child(
        &mut self,
        parent: &Inode,
        child: OsString,
        hashed_name: String,
        increment: bool,
    ) -> Result<Inode, String> {
        self.mapper
            .insert_child(parent, child, |data: ValueCreatorParams<InodeData>| {
                if data.existing_data.is_some() {
                    panic!("Try to insert an already existing child")
                }
                InodeData {
                    hashed_name: hashed_name.clone(),
                    nlookup: AtomicU64::new(if increment { 1 } else { 0 }),
                }
            })
            .map_err(|_| String::from("Could not add child"))
    }

    pub fn forget(&mut self, inode: &Inode, nlookup: u64) {
        if let Some(inode_info) = self.mapper.get(inode) {
            inode_info.data.nlookup.fetch_sub(nlookup, Ordering::SeqCst);
            if inode_info.data.nlookup.load(Ordering::SeqCst) <= 0 {
                self.mapper.remove(inode);
            }
        }
    }
}
