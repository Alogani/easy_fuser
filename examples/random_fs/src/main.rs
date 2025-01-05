#![doc = include_str!("../README.md")]

use easy_fuser::prelude::*;
use easy_fuser::templates::DefaultFuseHandler;
use rand::rngs::ThreadRng;
use rand::Rng;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct RandomFS {
    inner: DefaultFuseHandler,
}

const ROOT_ATTR: (Inode, FileAttribute) = (
    ROOT_INODE,
    FileAttribute {
        size: 0,
        blocks: 1,
        atime: UNIX_EPOCH,
        mtime: UNIX_EPOCH,
        ctime: UNIX_EPOCH,
        crtime: UNIX_EPOCH,
        kind: FileKind::Directory,
        perm: 0o777,
        nlink: 2,
        uid: 0,
        gid: 0,
        rdev: 0,
        flags: 0,
        blksize: 512,
        ttl: None,
        generation: None,
    },
);

impl RandomFS {
    pub fn new() -> Self {
        Self {
            inner: DefaultFuseHandler::new(),
        }
    }

    fn random_inode(rng: &mut ThreadRng) -> Inode {
        Inode::from(rng.gen::<u64>())
    }

    fn random_string(rng: &mut ThreadRng, len: usize) -> String {
        (0..len)
            .map(|_| rng.gen_range(b'a'..=b'z') as char)
            .collect()
    }

    fn random_data(rng: &mut ThreadRng, lines: usize) -> Vec<u8> {
        let line_length = rng.gen_range(10..50);
        (0..lines)
            .map(|_| format!("{}\n", Self::random_string(rng, line_length)))
            .collect::<String>()
            .into_bytes()
    }
}

impl FuseHandler<Inode> for RandomFS {
    fn get_inner(&self) -> &dyn FuseHandler<Inode> {
        &self.inner
    }

    fn access(&self, _req: &RequestInfo, _file_id: Inode, _mask: AccessMask) -> FuseResult<()> {
        Ok(())
    }

    fn create(
        &self,
        _req: &RequestInfo,
        _parent: Inode,
        _name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: OpenFlags,
    ) -> Result<
        (
            OwnedFileHandle,
            (Inode, FileAttribute),
            FUSEOpenResponseFlags,
        ),
        PosixError,
    > {
        let mut rng = rand::thread_rng();
        let ino = Self::random_inode(&mut rng);
        let attr = self.getattr(_req, ino.clone(), None)?;
        Ok((
            // Safe because we won't release it
            unsafe { OwnedFileHandle::from_raw(0) },
            (ino, attr),
            FUSEOpenResponseFlags::empty(),
        ))
    }

    fn getattr(
        &self,
        _req: &RequestInfo,
        ino: Inode,
        _fh: Option<BorrowedFileHandle>,
    ) -> FuseResult<FileAttribute> {
        if ino == ROOT_INODE {
            return Ok(ROOT_ATTR.1);
        }
        let mut rng = rand::thread_rng();
        let now = SystemTime::now();

        let attr = FileAttribute {
            size: rng.gen_range(0..10000),
            blocks: rng.gen_range(1..20),
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: if ino == ROOT_INODE {
                FileKind::Directory
            } else {
                if rng.gen_bool(0.7) {
                    FileKind::RegularFile
                } else {
                    FileKind::Directory
                }
            },
            perm: 0o755,
            nlink: rng.gen_range(1..5),
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
            blksize: 4096,
            ttl: None,
            generation: None,
        };

        Ok(attr)
    }

    fn lookup(
        &self,
        _req: &RequestInfo,
        _parent: Inode,
        _name: &OsStr,
    ) -> FuseResult<(Inode, FileAttribute)> {
        let mut rng = rand::thread_rng();
        let ino = Self::random_inode(&mut rng);
        let attr = self.getattr(_req, ino.clone(), None)?;
        Ok((ino, attr))
    }

    fn mkdir(
        &self,
        _req: &RequestInfo,
        _parent: Inode,
        _name: &OsStr,
        _mode: u32,
        _umask: u32,
    ) -> FuseResult<(Inode, FileAttribute)> {
        let mut rng = rand::thread_rng();
        let ino = Self::random_inode(&mut rng);
        let attr = self.getattr(_req, ino.clone(), None)?;
        Ok((ino, attr))
    }

    fn read(
        &self,
        _req: &RequestInfo,
        _ino: Inode,
        _fh: BorrowedFileHandle,
        offset: SeekFrom,
        size: u32,
        _flags: FUSEOpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        let mut rng = rand::thread_rng();
        let lines = rng.gen_range(0..81);
        let data = Self::random_data(&mut rng, lines);

        let offset = match offset {
            SeekFrom::Start(o) => o as usize,
            _ => return Err(ErrorKind::InvalidArgument.to_error("Invalid offset")),
        };

        Ok(data[offset..].iter().take(size as usize).cloned().collect())
    }

    fn readdir(
        &self,
        _req: &RequestInfo,
        ino: Inode,
        _fh: BorrowedFileHandle,
    ) -> FuseResult<Vec<(OsString, (Inode, FileKind))>> {
        let mut rng = rand::thread_rng();
        let count = rng.gen_range(0..13);
        let mut entries = vec![
            (OsString::from("."), (ino, FileKind::Directory)),
            (
                OsString::from(".."),
                (Self::random_inode(&mut rng), FileKind::Directory),
            ),
        ];

        for _ in 0..count {
            let lines = rng.gen_range(0..10);
            let name = OsString::from(Self::random_string(&mut rng, lines));
            let kind = if rng.gen_bool(0.7) {
                FileKind::RegularFile
            } else {
                FileKind::Directory
            };
            entries.push((name, (Self::random_inode(&mut rng), kind)));
        }

        Ok(entries)
    }

    fn rmdir(&self, _req: &RequestInfo, _parent_id: Inode, _name: &OsStr) -> FuseResult<()> {
        Ok(())
    }

    fn setattr(
        &self,
        req: &RequestInfo,
        file_id: Inode,
        _attrs: SetAttrRequest,
    ) -> FuseResult<FileAttribute> {
        self.getattr(req, file_id, None)
    }

    fn write(
        &self,
        _req: &RequestInfo,
        _ino: Inode,
        _fh: BorrowedFileHandle,
        _offset: SeekFrom,
        data: Vec<u8>,
        _write_flags: FUSEWriteFlags,
        _flags: OpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<u32> {
        Ok(data.len() as u32)
    }

    fn unlink(&self, _req: &RequestInfo, _parent_id: Inode, _name: &OsStr) -> FuseResult<()> {
        Ok(())
    }
}

fn main() {
    let mountpoint = std::env::args()
        .nth(1)
        .expect("Usage: random_fs <MOUNTPOINT>");
    let options = vec![
        MountOption::RW,
        MountOption::FSName("random_fs".to_string()),
    ];

    let fs = RandomFS::new();

    println!("Mounting filesystem...");
    easy_fuser::mount(fs, Path::new(&mountpoint), &options, 1).unwrap();
}
