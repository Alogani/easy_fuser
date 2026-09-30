use easy_fuser::fuse_presets::catalog_fs::{CatalogFileHandle, CatalogFs, Entry, EntryStore};
use easy_fuser::fuse_presets::{StatelessHandler, UnimplementedFuseHandler};
use easy_fuser::fuse_serial::prelude::*;
use easy_fuser::storage::{BlobKeyEncoder, EncodedBlobStore, FileStore};
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::ops::Range;
use std::path::Path;
use std::time::UNIX_EPOCH;

const ROOT: u64 = 0;

#[derive(Clone)]
struct CatalogEntry {
    id: u64,
    attr: FileAttribute,
    blob: Option<u64>,
}
impl Entry for CatalogEntry {
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

struct Edge {
    parent: u64,
    name: OsString,
    child: u64,
    kind: FileKind,
}
struct InMemoryEntryStore {
    entries: HashMap<u64, CatalogEntry>,
    edges: Vec<Edge>,
}
impl EntryStore for InMemoryEntryStore {
    type Entry = CatalogEntry;
    fn get(&self, id: &u64) -> FuseResult<Option<CatalogEntry>> {
        Ok(self.entries.get(id).cloned())
    }
    fn lookup(&self, parent: &u64, name: &OsStr) -> FuseResult<Option<CatalogEntry>> {
        Ok(self
            .edges
            .iter()
            .find(|edge| edge.parent == *parent && edge.name == name)
            .and_then(|edge| self.entries.get(&edge.child))
            .cloned())
    }
    fn children(&self, parent: &u64) -> FuseResult<Vec<(OsString, u64, FileKind)>> {
        Ok(self
            .edges
            .iter()
            .filter(|edge| edge.parent == *parent)
            .map(|edge| (edge.name.clone(), edge.child, edge.kind))
            .collect())
    }
}

struct ExampleBlobKeyEncoder;
impl BlobKeyEncoder for ExampleBlobKeyEncoder {
    type BlobId = u64;
    type Key = String;
    fn encode(&self, blob: &u64) -> FuseResult<String> {
        Ok(format!("blobs/{blob}"))
    }
}

struct InMemoryFileStore {
    files: HashMap<String, Vec<u8>>,
}
impl FileStore for InMemoryFileStore {
    type Key = String;
    fn read(&self, key: &String, range: Range<u64>) -> FuseResult<Vec<u8>> {
        let bytes = self
            .files
            .get(key)
            .ok_or_else(|| ErrorKind::FileNotFound.to_error("blob does not exist"))?;
        let start = usize::try_from(range.start)
            .unwrap_or(usize::MAX)
            .min(bytes.len());
        let end = usize::try_from(range.end)
            .unwrap_or(usize::MAX)
            .min(bytes.len());
        Ok(if start <= end {
            bytes[start..end].to_vec()
        } else {
            Vec::new()
        })
    }
}

struct MyFs {
    catalog:
        CatalogFs<InMemoryEntryStore, EncodedBlobStore<InMemoryFileStore, ExampleBlobKeyEncoder>>,
    directories: StatelessHandler<Inode>,
    unsupported: UnimplementedFuseHandler<Inode>,
}
impl FuseHandler for MyFs {
    type TId = Inode;
    type FileHandle = CatalogFileHandle<u64>;
    easy_fuser::delegate_fs! { catalog, [ getattr, lookup, open, read, readdir ] }
    easy_fuser::delegate_fs! { directories, [ forget, fsyncdir, opendir, releasedir ] }
    easy_fuser::delegate_fs! { unsupported, [ access, bmap, copy_file_range, create, fallocate, flush, fsync, getlk, getxattr, ioctl, link, listxattr, lseek, mkdir, mknod, readlink, removexattr, rename, rmdir, setattr, setlk, setxattr, statfs, symlink, unlink, write ] }
}

fn attr(kind: FileKind, size: u64) -> FileAttribute {
    FileAttribute {
        size,
        blocks: size.div_ceil(512),
        atime: UNIX_EPOCH,
        mtime: UNIX_EPOCH,
        ctime: UNIX_EPOCH,
        crtime: UNIX_EPOCH,
        kind,
        perm: if kind == FileKind::Directory {
            0o755
        } else {
            0o644
        },
        nlink: if kind == FileKind::Directory { 2 } else { 1 },
        uid: 0,
        gid: 0,
        rdev: 0,
        flags: 0,
        blksize: 512,
        ttl: None,
        generation: None,
    }
}

fn filesystem() -> MyFs {
    let entries = HashMap::from([
        (
            ROOT,
            CatalogEntry {
                id: ROOT,
                attr: attr(FileKind::Directory, 0),
                blob: None,
            },
        ),
        (
            1,
            CatalogEntry {
                id: 1,
                attr: attr(FileKind::RegularFile, 21),
                blob: Some(10),
            },
        ),
        (
            2,
            CatalogEntry {
                id: 2,
                attr: attr(FileKind::Directory, 0),
                blob: None,
            },
        ),
        (
            3,
            CatalogEntry {
                id: 3,
                attr: attr(FileKind::RegularFile, 18),
                blob: Some(11),
            },
        ),
    ]);
    let edges = vec![
        Edge {
            parent: ROOT,
            name: "hello.txt".into(),
            child: 1,
            kind: FileKind::RegularFile,
        },
        Edge {
            parent: ROOT,
            name: "models".into(),
            child: 2,
            kind: FileKind::Directory,
        },
        Edge {
            parent: 2,
            name: "model.bin".into(),
            child: 3,
            kind: FileKind::RegularFile,
        },
    ];
    let entry_store = InMemoryEntryStore { entries, edges };
    let file_store = InMemoryFileStore {
        files: HashMap::from([
            ("blobs/10".into(), b"Hello from CatalogFs!".to_vec()),
            ("blobs/11".into(), b"example model data".to_vec()),
        ]),
    };
    let blobs = EncodedBlobStore::new(file_store, ExampleBlobKeyEncoder);
    MyFs {
        catalog: CatalogFs::new(ROOT, entry_store, blobs),
        directories: StatelessHandler::new(),
        unsupported: UnimplementedFuseHandler::new(),
    }
}

fn main() {
    let mountpoint = std::env::args()
        .nth(1)
        .expect("Usage: catalog_fs <MOUNTPOINT>");
    let options = vec![MountOption::RO, MountOption::FSName("catalog_fs".into())];
    mount(
        filesystem(),
        Path::new(&mountpoint),
        &options,
        Some(MountThreads::same(1)),
    )
    .unwrap();
}
