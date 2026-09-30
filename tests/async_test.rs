#![cfg(feature = "async")]

mod common;

use easy_fuser::fuse_async::prelude::*;
use easy_fuser::fuse_presets::mirror_fs::*;
use easy_fuser::fuse_presets::{StatelessHandler, UnimplementedFuseHandler};
#[cfg(feature = "io_uring")]
use easy_fuser_macro::delegate_fs_async as delegate_mirror;
use easy_fuser_macro::delegate_fs_sync_to_async;
#[cfg(not(feature = "io_uring"))]
use easy_fuser_macro::delegate_fs_sync_to_async as delegate_mirror;

use async_trait::async_trait;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

#[cfg(feature = "io_uring")]
type AsyncMirror = MirrorFsAsync;
#[cfg(not(feature = "io_uring"))]
type AsyncMirror = MirrorFs;

struct MyAsyncFs {
    mirror_fs: AsyncMirror,
    unimplemented: UnimplementedFuseHandler<PathBuf>,
    safe_defaults: StatelessHandler<PathBuf>,
}

#[async_trait]
impl FuseHandler for MyAsyncFs {
    type TId = PathBuf;
    type FileHandle = std::os::fd::OwnedFd;

    delegate_mirror! { mirror_fs, [
        flush, fsync, lseek, read, release,
        access, getattr, getxattr, listxattr, lookup, open, readdir, readlink,
        copy_file_range, fallocate, write,
        create, mkdir, mknod, removexattr, rename, rmdir, setattr, setxattr, symlink, unlink
    ]}

    delegate_fs_sync_to_async! { safe_defaults, [ forget, fsyncdir, opendir, releasedir ] }
    delegate_fs_sync_to_async! { unimplemented, [ bmap, getlk, ioctl, link, setlk, statfs ] }
}

#[test]
fn test_async_mirror_fs() {
    #[cfg(all(feature = "io_uring", target_os = "linux"))]
    if io_uring::IoUring::new(8).is_err() {
        // Some CI/container seccomp profiles disable io_uring_setup.
        return;
    }

    let mount_dir = TempDir::new().unwrap();
    let source_dir = TempDir::new().unwrap();

    let mntpoint = mount_dir.path().to_path_buf();
    let source_path = source_dir.path().to_path_buf();

    // Create a test file
    let test_file = source_path.join("test_async.txt");
    fs::write(&test_file, "Async FUSE test").unwrap();

    let mntpoint_clone = mntpoint.clone();
    let handle = std::thread::spawn(move || {
        let fs = MyAsyncFs {
            mirror_fs: AsyncMirror::new(source_path),
            unimplemented: UnimplementedFuseHandler::new(),
            safe_defaults: StatelessHandler::new(),
        };
        mount(fs, &mntpoint_clone, &[], Some(common::MOUNT_THREADS)).unwrap();
    });
    // Wait for the mount to finish
    let mnt_file = mntpoint.join("test_async.txt");
    let mut mounted = false;
    for _ in 0..100 {
        if mnt_file.exists() {
            mounted = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(mounted, "Mount timed out");

    {
        let content = fs::read_to_string(&mnt_file).unwrap();
        assert_eq!(content, "Async FUSE test");
    }
    fs::write(&mnt_file, "async write through the preset").unwrap();
    assert_eq!(
        fs::read_to_string(&test_file).unwrap(),
        "async write through the preset"
    );

    // Unmount
    let mut unmounted = false;
    for cmd_name in &["fusermount3", "fusermount", "umount"] {
        let mut cmd = std::process::Command::new(cmd_name);
        if cmd_name == &"umount" {
            cmd.arg(&mntpoint);
        } else {
            cmd.arg("-u").arg(&mntpoint);
        }
        if let Ok(status) = cmd.status() {
            if status.success() {
                unmounted = true;
                break;
            }
        }
    }
    assert!(unmounted, "Failed to unmount async filesystem");
    #[cfg(not(any(target_os = "freebsd")))]
    handle.join().unwrap();
    #[cfg(target_os = "freebsd")]
    let _ = handle.join();
}
