#![doc = include_str!("../README.md")]

use clap::Parser;
use ctrlc;
use easy_fuser::fuse_parallel::prelude::*;
use easy_fuser::fuse_presets::mirror_fs::*;
use easy_fuser::fuse_presets::{StatelessHandler, UnimplementedFuseHandler};
use std::path::PathBuf;
use std::process::exit;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Mount point for the mirror filesystem
    #[arg(short, long)]
    mntpoint: Option<PathBuf>,

    /// Source directory to mirror
    #[arg(short, long)]
    source_dir: Option<PathBuf>,

    /// Mount the filesystem as read-only
    #[arg(short, long)]
    read_only: bool,

    /// Enable verbose logging (trace level) and set RUST_BACKTRACE=full
    #[arg(short, long)]
    verbose: bool,

    /// Positional arguments: [SOURCE_DIR] [MOUNT_POINT]
    #[arg(required = false)]
    args: Vec<PathBuf>,
}

struct MyMirrorFs {
    mirror_fs: MirrorFs,
    unimplemented: UnimplementedFuseHandler<PathBuf>,
    safe_defaults: StatelessHandler<PathBuf>,
}

impl MyMirrorFs {
    fn new(source_path: PathBuf) -> Self {
        Self {
            mirror_fs: MirrorFs::new(source_path),
            unimplemented: UnimplementedFuseHandler::new(),
            safe_defaults: StatelessHandler::new(),
        }
    }

    fn source_dir(&self) -> &std::path::Path {
        self.mirror_fs.source_dir()
    }
}

impl FuseHandler for MyMirrorFs {
    type TId = PathBuf;
    type FileHandle = std::os::fd::OwnedFd;

    easy_fuser::delegate_fs! { mirror_fs, [
        flush, fsync, lseek, read, release,
        access, getattr, getxattr, listxattr, lookup, open, readdir, readlink,
        copy_file_range, fallocate, write,
        create, mkdir, mknod, removexattr, rename, rmdir, setattr, setxattr, symlink, unlink
    ]}

    easy_fuser::delegate_fs! { safe_defaults, [ forget, fsyncdir, opendir, releasedir ] }
    easy_fuser::delegate_fs! { unimplemented, [ bmap, getlk, ioctl, link, setlk, statfs ] }
}

struct MyMirrorFsReadOnly {
    mirror_fs: MirrorFsReadOnly,
    unimplemented: UnimplementedFuseHandler<PathBuf>,
    safe_defaults: StatelessHandler<PathBuf>,
}

impl MyMirrorFsReadOnly {
    fn new(source_path: PathBuf) -> Self {
        Self {
            mirror_fs: MirrorFsReadOnly::new(source_path),
            unimplemented: UnimplementedFuseHandler::new(),
            safe_defaults: StatelessHandler::new(),
        }
    }

    fn source_dir(&self) -> &std::path::Path {
        self.mirror_fs.source_dir()
    }
}

impl FuseHandler for MyMirrorFsReadOnly {
    type TId = PathBuf;
    type FileHandle = std::os::fd::OwnedFd;

    easy_fuser::delegate_fs! { mirror_fs, [
        flush, fsync, lseek, read, release,
        access, getattr, getxattr, listxattr, lookup, open, readdir, readlink
    ]}

    easy_fuser::delegate_fs! { safe_defaults, [ forget, fsyncdir, opendir, releasedir ] }
    easy_fuser::delegate_fs! { unimplemented, [ copy_file_range, fallocate, write, create, mkdir, mknod, removexattr, rename, rmdir, setattr, setxattr, symlink, unlink, bmap, getlk, ioctl, link, setlk, statfs ] }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Enable verbose logging and full backtrace if requested
    if args.verbose {
        unsafe { std::env::set_var("RUST_BACKTRACE", "full") };
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Trace)
            .try_init();
    } else {
        let _ = env_logger::try_init();
    }

    let (source_dir, mntpoint) = if !args.args.is_empty() {
        if args.args.len() != 2 {
            return Err(
                "Expected exactly two positional arguments: <SOURCE_DIR> <MOUNT_POINT>".into(),
            );
        }
        if args.source_dir.is_some() || args.mntpoint.is_some() {
            return Err("Cannot mix positional and named arguments".into());
        }
        (args.args[0].clone(), args.args[1].clone())
    } else {
        let source_dir = args.source_dir.ok_or("Source directory is required")?;
        let mntpoint = args.mntpoint.ok_or("Mount point is required")?;
        (source_dir, mntpoint)
    };

    // Ensure the mount point exists
    std::fs::create_dir_all(&mntpoint)?;

    // Set up the cleanup function
    let once_flag = Arc::new(AtomicBool::new(false));
    let cleanup = |mntpoint: &PathBuf, once_flag: &Arc<AtomicBool>| {
        if once_flag.clone().swap(true, Ordering::SeqCst) {
            return;
        }
        println!("Unmounting filesystem...");
        let mut unmounted = false;
        for cmd_name in &["fusermount3", "fusermount", "umount"] {
            let mut cmd = Command::new(cmd_name);
            if cmd_name == &"umount" {
                cmd.arg(mntpoint);
            } else {
                cmd.arg("-u").arg(mntpoint);
            }
            if let Ok(status) = cmd.status() {
                if status.success() {
                    unmounted = true;
                    break;
                }
            }
        }
        if !unmounted {
            println!("Warning: Failed to unmount using fusermount3, fusermount, or umount");
        }
    };

    // Set up Ctrl+C handler
    let mntpoint_ctrlc = mntpoint.clone();
    let onceflag_ctrlc = once_flag.clone();
    ctrlc::set_handler(move || {
        println!("Received Ctrl+C, unmounting...");
        cleanup(&mntpoint_ctrlc, &onceflag_ctrlc);
        exit(1);
    })?;

    if args.read_only {
        let fs = MyMirrorFsReadOnly::new(source_dir);
        println!("Mounting mirror filesystem in READ-ONLY mode...");
        println!("Mount point: {:?}", &mntpoint);
        println!("Source directory: {:?}", fs.source_dir());
        mount(fs, &mntpoint, &[], Some(1))?;
    } else {
        let fs = MyMirrorFs::new(source_dir);
        println!("Mounting mirror filesystem in READ-WRITE mode...");
        println!("Mount point: {:?}", &mntpoint);
        println!("Source directory: {:?}", fs.source_dir());
        mount(fs, &mntpoint, &[], Some(1))?;
    }

    // If we reach here, the filesystem has been unmounted normally
    cleanup(&mntpoint, &once_flag);

    Ok(())
}
