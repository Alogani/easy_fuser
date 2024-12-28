//! Largely inspired from: https://github.com/cberner/fuser/blob/v0.15.1/examples/hello.rs
//! 
//! Give you an example of how to create a simple FUSE filesystem in Rust without using templates
//! Use templates if you want to jumpstart your implementation!
//! 
//! It uses Inode as FileIdType for teaching purpose,
//! but many user will feel more conformtable using PathBuf (see the other examples)

use easy_fuser::prelude::*;
use easy_fuser::templates::DefaultFuseHandler;
use std::ffi::{OsStr, OsString};
use std::time::{Duration, UNIX_EPOCH};

const TTL: Duration = Duration::from_secs(1); // 1 second

const HELLO_DIR_ATTR: (Inode, FileAttribute) = (
    ROOT_INODE,
    FileAttribute {
        size: 0,
        blocks: 0,
        atime: UNIX_EPOCH,
        mtime: UNIX_EPOCH,
        ctime: UNIX_EPOCH,
        crtime: UNIX_EPOCH,
        kind: FileKind::Directory,
        perm: 0o755,
        nlink: 2,
        uid: 501,
        gid: 20,
        rdev: 0,
        flags: 0,
        blksize: 512,
        ttl: None,
        generation: None,
    }
);

const HELLO_TXT_CONTENT: &str = "Hello World!\n";

const HELLO_TXT_ATTR: (Inode, FileAttribute) = (
    Inode::from(2),
    FileAttribute {
        size: 13,
        blocks: 1,
        atime: UNIX_EPOCH,
        mtime: UNIX_EPOCH,
        ctime: UNIX_EPOCH,
        crtime: UNIX_EPOCH,
        kind: FileKind::RegularFile,
        perm: 0o644,
        nlink: 1,
        uid: 501,
        gid: 20,
        rdev: 0,
        flags: 0,
        blksize: 512,
        ttl: None,
        generation: None,
    }
);

struct HelloFS {
    // To avoid implemeting all fuse methods
    inner: DefaultFuseHandler,
}

impl HelloFS {
    fn new() -> Self {
        Self {
            inner: DefaultFuseHandler::new(),
        }
    }
}

impl FuseHandler<Inode> for HelloFS {
    fn get_inner(&self) -> &dyn FuseHandler<Inode> {
        &self.inner
    }

    fn get_default_ttl(&self) -> Duration {
        TTL
    }

    fn lookup(&self, _req: &RequestInfo, parent_id: Inode, name: &OsStr) -> FuseResult<(Inode, FileAttribute)> {
        if parent_id == ROOT_INODE && name == "hello.txt" {
            Ok(HELLO_TXT_ATTR)
        } else {
            // Or PosixError::new(ErrorKind::FileNotFound, "")
            Err(ErrorKind::FileNotFound.to_error(""))
        }
    }

    fn getattr(
        &self,
        _req: &RequestInfo,
        file_id: Inode,
        _file_handle: Option<FileHandle>,
    ) -> FuseResult<FileAttribute> {
        match file_id {
            ROOT_INODE => Ok(HELLO_DIR_ATTR.1),
            inode if inode == HELLO_TXT_ATTR.0 => Ok(HELLO_TXT_ATTR.1),
            _ => Err(ErrorKind::FileNotFound.to_error("")),
        }
    }

    fn read(
        &self,
        _req: &RequestInfo,
        file_id: Inode,
        _file_handle: FileHandle,
        seek: SeekFrom,
        size: u32,
        _flags: FUSEOpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        if file_id == HELLO_TXT_ATTR.0 {
            let offset = match seek {
                SeekFrom::Start(offset) => offset as usize,
                _ => return Err(ErrorKind::InvalidArgument.to_error(format!("{:?}", seek))),
            };
            let content = HELLO_TXT_CONTENT.as_bytes();
            Ok(content[offset..].iter().take(size as usize).cloned().collect())
        } else {
            Err(ErrorKind::FileNotFound.to_error(""))
        }
    }

    fn readdir(
        &self,
        _req: &RequestInfo,
        file_id: Inode,
        _file_handle: FileHandle,
    ) -> FuseResult<Vec<(OsString, (Inode, FileKind))>> {
        if file_id == ROOT_INODE {
            Ok(vec![
                (OsString::from("."), (HELLO_DIR_ATTR.0, HELLO_DIR_ATTR.1.kind)),
                (OsString::from(".."), (HELLO_DIR_ATTR.0, HELLO_DIR_ATTR.1.kind)),
                (OsString::from("hello.txt"), (HELLO_DIR_ATTR.0, HELLO_DIR_ATTR.1.kind)),
            ])
        } else {
            Err(ErrorKind::FileNotFound.to_error(""))
        }
    }
}

fn main() {
    #[cfg(feature = "logging")]
    std::env::set_var("RUST_BACKTRACE", "full");
    #[cfg(feature = "logging")]
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Trace)
        .try_init();


    let mountpoint = std::env::args().nth(1).expect("Usage: hello <MOUNTPOINT>");
    let options = vec![MountOption::RO, MountOption::FSName("hello".to_string())];
    
    easy_fuser::mount(HelloFS::new(), mountpoint.as_ref(), &options, 1).unwrap();
}