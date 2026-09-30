//! Read-only FUSE adapter for metadata/namespace and logical blob stores.
//!
//! [`EntryStore`] supplies object metadata and namespace relationships;
//! [`crate::storage::BlobStore`] supplies file contents by logical blob ID.
//! `CatalogFs` adapts these independent layers to FUSE and assigns stable
//! inodes to logical entry IDs. The preset is initially read-only.

use crate::inode_mapping::{InodeMapper, InsertError, LinkError};
use crate::storage::BlobStore;
use crate::types::*;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::hash::Hash;
use std::io::SeekFrom;
use std::ops::Range;
use std::sync::Mutex;

/// A metadata object independent of any name or parent in the namespace.
pub trait Entry {
    type Id: Clone + Eq + Hash + Send + Sync + 'static;
    type BlobId: Clone + Send + Sync + 'static;

    fn id(&self) -> &Self::Id;
    fn attributes(&self) -> FileAttribute;
    fn blob_id(&self) -> Option<&Self::BlobId>;
}

/// Provides object metadata and the separate `(parent, name) -> entry` namespace.
pub trait EntryStore: Send + Sync + 'static {
    type Entry: Entry;

    fn get(&self, id: &<Self::Entry as Entry>::Id) -> FuseResult<Option<Self::Entry>>;

    fn lookup(
        &self,
        parent: &<Self::Entry as Entry>::Id,
        name: &OsStr,
    ) -> FuseResult<Option<Self::Entry>>;

    fn children(
        &self,
        parent: &<Self::Entry as Entry>::Id,
    ) -> FuseResult<Vec<(OsString, <Self::Entry as Entry>::Id, FileKind)>>;
}

/// Lightweight per-open state used by [`CatalogFs`]; reads need no further
/// metadata lookup. Applications name this type as the `FuseHandler::FileHandle`
/// when delegating open/read to a catalog preset.
#[derive(Debug)]
pub struct CatalogFileHandle<B> {
    blob_id: B,
    size: u64,
}

struct CatalogMapping<Id> {
    mapper: InodeMapper<Id>,
    inodes: HashMap<Id, Inode>,
}

/// Read-only FUSE preset combining namespace metadata with logical blob reads.
pub struct CatalogFs<E, B>
where
    E: EntryStore,
    B: BlobStore<BlobId = <<E as EntryStore>::Entry as Entry>::BlobId>,
{
    entries: E,
    blobs: B,
    mapping: Mutex<CatalogMapping<<<E as EntryStore>::Entry as Entry>::Id>>,
}

impl<E, B> CatalogFs<E, B>
where
    E: EntryStore,
    <E::Entry as Entry>::Id: Hash,
    B: BlobStore<BlobId = <E::Entry as Entry>::BlobId>,
{
    /// Creates a read-only catalog filesystem with the supplied logical root ID.
    pub fn new(root_id: <E::Entry as Entry>::Id, entries: E, blobs: B) -> Self {
        Self {
            entries,
            blobs,
            mapping: Mutex::new(CatalogMapping {
                mapper: InodeMapper::new(root_id.clone()),
                inodes: HashMap::from([(root_id, ROOT_INODE)]),
            }),
        }
    }

    fn mapping_error() -> PosixError {
        ErrorKind::InputOutputError.to_error("catalog inode mapping lock is poisoned")
    }

    fn missing_entry() -> PosixError {
        ErrorKind::FileNotFound.to_error("catalog entry does not exist")
    }

    fn inode_for_child(
        mapping: &mut CatalogMapping<<E::Entry as Entry>::Id>,
        parent: &Inode,
        name: OsString,
        id: <E::Entry as Entry>::Id,
    ) -> FuseResult<Inode> {
        if let Some(inode) = mapping.inodes.get(&id).copied() {
            if mapping.mapper.lookup(parent, &name).is_none() {
                mapping
                    .mapper
                    .link(&inode, parent, name)
                    .map_err(|error| match error {
                        LinkError::InodeNotFound
                        | LinkError::ParentNotFound
                        | LinkError::NameExists => Self::mapping_error(),
                    })?;
            }
            return Ok(inode);
        }

        let id_for_mapper = id.clone();
        let inode = mapping
            .mapper
            .insert_child(parent, name, |_| id_for_mapper.clone())
            .map_err(|InsertError::ParentNotFound| Self::missing_entry())?;
        mapping.inodes.insert(id, inode);
        Ok(inode)
    }

    fn entry_id(&self, inode: Inode) -> FuseResult<<E::Entry as Entry>::Id> {
        let mapping = self.mapping.lock().map_err(|_| Self::mapping_error())?;
        mapping
            .mapper
            .get(&inode)
            .map(|info| info.data.clone())
            .ok_or_else(Self::missing_entry)
    }

    /// Returns the configured root inode (always [`ROOT_INODE`]).
    pub fn root_inode(&self) -> Inode {
        ROOT_INODE
    }

    pub fn lookup(
        &self,
        _req: &RequestInfo,
        parent: Inode,
        name: &OsStr,
    ) -> FuseResult<(Inode, FileAttribute)> {
        let parent_id = self.entry_id(parent)?;
        let entry = self
            .entries
            .lookup(&parent_id, name)?
            .ok_or_else(Self::missing_entry)?;
        let id = entry.id().clone();
        let attributes = entry.attributes();
        let mut mapping = self.mapping.lock().map_err(|_| Self::mapping_error())?;
        let inode = Self::inode_for_child(&mut mapping, &parent, name.to_owned(), id)?;
        Ok((inode, attributes))
    }

    pub fn getattr(
        &self,
        _req: &RequestInfo,
        inode: Inode,
        _file_handle: Option<&mut CatalogFileHandle<<E::Entry as Entry>::BlobId>>,
    ) -> FuseResult<FileAttribute> {
        let id = self.entry_id(inode)?;
        self.entries
            .get(&id)?
            .map(|entry| entry.attributes())
            .ok_or_else(Self::missing_entry)
    }

    pub fn readdir(
        &self,
        _req: &RequestInfo,
        inode: Inode,
        _file_handle: BorrowedFileHandle<'_>,
    ) -> FuseResult<Vec<(OsString, (Inode, FileKind))>> {
        let parent_id = self.entry_id(inode)?;
        let children = self.entries.children(&parent_id)?;
        let mut mapping = self.mapping.lock().map_err(|_| Self::mapping_error())?;
        children
            .into_iter()
            .map(|(name, id, kind)| {
                let child_inode = Self::inode_for_child(&mut mapping, &inode, name.clone(), id)?;
                Ok((name, (child_inode, kind)))
            })
            .collect()
    }

    pub fn open(
        &self,
        _req: &RequestInfo,
        inode: Inode,
        _flags: OpenFlags,
    ) -> FuseResult<(CatalogFileHandle<<E::Entry as Entry>::BlobId>, FopenFlags)> {
        let id = self.entry_id(inode)?;
        let entry = self.entries.get(&id)?.ok_or_else(Self::missing_entry)?;
        let attributes = entry.attributes();
        if attributes.kind == FileKind::Directory {
            return Err(
                ErrorKind::IsADirectory.to_error("cannot open a catalog directory as a file")
            );
        }
        if attributes.kind != FileKind::RegularFile {
            return Err(ErrorKind::InvalidArgument.to_error("catalog entry is not a regular file"));
        }
        let blob_id = entry.blob_id().cloned().ok_or_else(|| {
            ErrorKind::InputOutputError.to_error("regular catalog file has no blob ID")
        })?;
        Ok((
            CatalogFileHandle {
                blob_id,
                size: attributes.size,
            },
            FopenFlags::empty(),
        ))
    }

    pub fn read(
        &self,
        _req: &RequestInfo,
        _inode: Inode,
        file_handle: Option<&mut CatalogFileHandle<<E::Entry as Entry>::BlobId>>,
        seek: SeekFrom,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        let handle = file_handle
            .ok_or_else(|| ErrorKind::BadFileDescriptor.to_error("missing catalog file handle"))?;
        let offset = match seek {
            SeekFrom::Start(offset) => offset,
            _ => {
                return Err(
                    ErrorKind::IllegalSeek.to_error("catalog reads require an absolute offset")
                );
            }
        };
        if offset >= handle.size {
            return Ok(Vec::new());
        }
        let read_len = u64::from(size).min(handle.size - offset);
        let requested_end = offset
            .checked_add(read_len)
            .ok_or_else(|| ErrorKind::ValueTooLarge.to_error("catalog read range overflow"))?;
        let range: Range<u64> = offset..requested_end;
        self.blobs.read(&handle.blob_id, range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RequestId;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::UNIX_EPOCH;

    #[derive(Clone)]
    struct TestEntry {
        id: u64,
        attr: FileAttribute,
        blob: Option<u64>,
    }

    impl Entry for TestEntry {
        type Id = u64;
        type BlobId = u64;
        fn id(&self) -> &u64 {
            &self.id
        }
        fn attributes(&self) -> FileAttribute {
            self.attr.clone()
        }
        fn blob_id(&self) -> Option<&u64> {
            self.blob.as_ref()
        }
    }

    #[derive(Clone)]
    struct Edge {
        name: OsString,
        parent: u64,
        child: u64,
        kind: FileKind,
    }

    struct TestEntries {
        entries: HashMap<u64, TestEntry>,
        edges: Vec<Edge>,
    }

    impl EntryStore for TestEntries {
        type Entry = TestEntry;
        fn get(&self, id: &u64) -> FuseResult<Option<TestEntry>> {
            Ok(self.entries.get(id).cloned())
        }
        fn lookup(&self, parent: &u64, name: &OsStr) -> FuseResult<Option<TestEntry>> {
            Ok(self
                .edges
                .iter()
                .find(|e| e.parent == *parent && e.name == name)
                .and_then(|e| self.entries.get(&e.child))
                .cloned())
        }
        fn children(&self, parent: &u64) -> FuseResult<Vec<(OsString, u64, FileKind)>> {
            Ok(self
                .edges
                .iter()
                .filter(|e| e.parent == *parent)
                .map(|e| (e.name.clone(), e.child, e.kind))
                .collect())
        }
    }

    struct TestBlobs {
        seen: Mutex<Vec<(u64, Range<u64>)>>,
        bytes: Vec<u8>,
    }
    impl BlobStore for TestBlobs {
        type BlobId = u64;
        fn read(&self, blob: &u64, range: Range<u64>) -> FuseResult<Vec<u8>> {
            self.seen.lock().unwrap().push((*blob, range.clone()));
            Ok(self.bytes[range.start as usize..range.end as usize].to_vec())
        }
    }

    fn attr(kind: FileKind, size: u64) -> FileAttribute {
        FileAttribute {
            size,
            blocks: 1,
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind,
            perm: 0o755,
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
            blksize: 512,
            ttl: None,
            generation: None,
        }
    }

    fn fixture() -> CatalogFs<TestEntries, TestBlobs> {
        let entries = HashMap::from([
            (
                0,
                TestEntry {
                    id: 0,
                    attr: attr(FileKind::Directory, 0),
                    blob: None,
                },
            ),
            (
                1,
                TestEntry {
                    id: 1,
                    attr: attr(FileKind::RegularFile, 5),
                    blob: Some(10),
                },
            ),
            (
                2,
                TestEntry {
                    id: 2,
                    attr: attr(FileKind::Directory, 0),
                    blob: None,
                },
            ),
        ]);
        let edges = vec![
            Edge {
                name: "one".into(),
                parent: 0,
                child: 1,
                kind: FileKind::RegularFile,
            },
            Edge {
                name: "alias".into(),
                parent: 0,
                child: 1,
                kind: FileKind::RegularFile,
            },
            Edge {
                name: "folder".into(),
                parent: 0,
                child: 2,
                kind: FileKind::Directory,
            },
        ];
        CatalogFs::new(
            0,
            TestEntries { entries, edges },
            TestBlobs {
                seen: Mutex::new(Vec::new()),
                bytes: b"hello".to_vec(),
            },
        )
    }

    fn req() -> RequestInfo {
        RequestInfo {
            id: RequestId(0),
            uid: 0,
            gid: 0,
            pid: 0,
        }
    }

    #[test]
    fn root_lookup_links_and_readdir_use_stable_inodes() {
        let fs = fixture();
        assert_eq!(fs.entry_id(fs.root_inode()).unwrap(), 0);
        let (first, found) = fs.lookup(&req(), ROOT_INODE, OsStr::new("one")).unwrap();
        assert_eq!(found.size, 5);
        let (again, _) = fs.lookup(&req(), ROOT_INODE, OsStr::new("one")).unwrap();
        let (alias, _) = fs.lookup(&req(), ROOT_INODE, OsStr::new("alias")).unwrap();
        assert_eq!(first, again);
        assert_eq!(first, alias);
        let children = fs
            .readdir(&req(), ROOT_INODE, unsafe {
                BorrowedFileHandle::from_raw(0)
            })
            .unwrap();
        assert_eq!(children.len(), 3);
        assert!(children
            .iter()
            .any(|(name, (inode, _))| name == "alias" && *inode == first));
    }

    #[test]
    fn getattr_open_and_read_use_blob_id_and_clamp_at_eof() {
        let fs = fixture();
        let (inode, _) = fs.lookup(&req(), ROOT_INODE, OsStr::new("one")).unwrap();
        assert_eq!(fs.getattr(&req(), inode, None).unwrap().size, 5);
        let (mut handle, _) = fs.open(&req(), inode, OpenFlags(0)).unwrap();
        assert_eq!(
            fs.read(
                &req(),
                inode,
                Some(&mut handle),
                SeekFrom::Start(2),
                10,
                OpenFlags(0),
                None
            )
            .unwrap(),
            b"llo"
        );
        assert_eq!(
            fs.read(
                &req(),
                inode,
                Some(&mut handle),
                SeekFrom::Start(5),
                4,
                OpenFlags(0),
                None
            )
            .unwrap(),
            b""
        );
        assert_eq!(*fs.blobs.seen.lock().unwrap(), vec![(10, 2..5)]);
    }

    #[test]
    fn missing_entry_is_not_found_and_directories_are_not_files() {
        let fs = fixture();
        assert_eq!(
            fs.lookup(&req(), ROOT_INODE, OsStr::new("absent"))
                .unwrap_err()
                .kind(),
            ErrorKind::FileNotFound
        );
        assert_eq!(
            fs.getattr(&req(), fuser::INodeNo(999), None)
                .unwrap_err()
                .kind(),
            ErrorKind::FileNotFound
        );
        let (dir, _) = fs.lookup(&req(), ROOT_INODE, OsStr::new("folder")).unwrap();
        assert_eq!(
            fs.open(&req(), dir, OpenFlags(0)).unwrap_err().kind(),
            ErrorKind::IsADirectory
        );
    }
}
