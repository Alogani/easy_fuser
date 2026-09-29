#![cfg(any(feature = "serial", feature = "parallel"))]

#[cfg(all(feature = "parallel", not(feature = "serial")))]
use easy_fuser::fuse_parallel::prelude::*;
#[cfg(feature = "serial")]
use easy_fuser::fuse_serial::prelude::*;

use easy_fuser::fuse_presets::{StatelessHandler, UnimplementedFuseHandler};
use easy_fuser::unix_fs;
use easy_fuser_macro::delegate_fs;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct LinkedFs {
    source: PathBuf,
    defaults: UnimplementedFuseHandler<MappedInode>,
    safe_defaults: StatelessHandler<MappedInode>,
}

impl LinkedFs {
    fn source_path(&self, id: MappedInode) -> FuseResult<PathBuf> {
        let path = id
            .paths()
            .into_iter()
            .next()
            .ok_or_else(|| PosixError::new(libc::ENOENT, "inode has no path"))?;
        Ok(self.source.join(path))
    }
}

impl FuseHandler for LinkedFs {
    type TId = MappedInode;

    fn getattr(
        &self,
        _req: &RequestInfo,
        id: MappedInode,
        _fh: Option<BorrowedFileHandle<'_>>,
    ) -> FuseResult<FileAttribute> {
        unix_fs::lookup(&self.source_path(id)?)
    }

    fn lookup(
        &self,
        _req: &RequestInfo,
        parent: MappedInode,
        name: &OsStr,
    ) -> FuseResult<FileAttribute> {
        unix_fs::lookup(&self.source_path(parent)?.join(name))
    }

    fn link(
        &self,
        _req: &RequestInfo,
        id: MappedInode,
        newparent: MappedInode,
        newname: &OsStr,
    ) -> FuseResult<FileAttribute> {
        let old = self.source_path(id)?;
        let new = self.source_path(newparent)?.join(newname);
        fs::hard_link(&old, &new)?;
        unix_fs::lookup(&new)
    }

    fn unlink(&self, _req: &RequestInfo, parent: MappedInode, name: &OsStr) -> FuseResult<()> {
        fs::remove_file(self.source_path(parent)?.join(name))?;
        Ok(())
    }

    fn rename(
        &self,
        _req: &RequestInfo,
        parent: MappedInode,
        name: &OsStr,
        newparent: MappedInode,
        newname: &OsStr,
        _flags: RenameFlags,
    ) -> FuseResult<()> {
        fs::rename(
            self.source_path(parent)?.join(name),
            self.source_path(newparent)?.join(newname),
        )?;
        Ok(())
    }

    delegate_fs! { safe_defaults, [ forget, fsyncdir, opendir, releasedir ] }
    delegate_fs! { defaults, [ access, bmap, copy_file_range, create, fallocate, flush, fsync, getlk, getxattr, ioctl, listxattr, lseek, mkdir, mknod, open, read, readdir, readlink, release, removexattr, rmdir, setlk, setattr, setxattr, statfs, symlink, write ] }
}

#[test]
fn mounted_hard_links_share_one_inode() {
    let source = TempDir::new().unwrap();
    let mountpoint = TempDir::new().unwrap();
    fs::write(source.path().join("a"), b"shared").unwrap();
    let session = spawn_mount(
        LinkedFs {
            source: source.path().to_path_buf(),
            defaults: UnimplementedFuseHandler::new(),
            safe_defaults: StatelessHandler::new(),
        },
        Path::new(mountpoint.path()),
        &[],
        Some(2),
    )
    .unwrap();

    let a = mountpoint.path().join("a");
    let b = mountpoint.path().join("b");
    let c = mountpoint.path().join("c");
    let original_inode = fs::metadata(&a).unwrap().ino();
    fs::hard_link(&a, &b).unwrap();
    assert_eq!(fs::metadata(&b).unwrap().ino(), original_inode);
    fs::remove_file(&a).unwrap();
    assert_eq!(fs::metadata(&b).unwrap().ino(), original_inode);
    fs::rename(&b, &c).unwrap();
    assert_eq!(fs::metadata(&c).unwrap().ino(), original_inode);
    assert_eq!(fs::read(source.path().join("c")).unwrap(), b"shared");
    session.join().unwrap();
}
