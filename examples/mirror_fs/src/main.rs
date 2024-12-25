use std::path::PathBuf;
use std::process::exit;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use clap::Parser;
use ctrlc;
use easy_fuser::prelude::*;
use easy_fuser::templates::DefaultFuseHandler;
use easy_fuser::templates::MirrorFs;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Mount point for the mirror filesystem
    #[arg(short, long)]
    mntpoint: Option<PathBuf>,

    /// Source directory to mirror
    #[arg(short, long)]
    source_dir: Option<PathBuf>,

    /// Positional arguments: [SOURCE_DIR] [MOUNT_POINT]
    #[arg(required = false)]
    args: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let (source_dir, mntpoint) = if !args.args.is_empty() {
        if args.args.len() != 2 {
            return Err("Expected exactly two positional arguments: <SOURCE_DIR> <MOUNT_POINT>".into());
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
        let _ = Command::new("fusermount")
            .arg("-u")
            .arg(mntpoint)
            .status();
    };

    // Set up Ctrl+C handler
    let mntpoint_ctrlc = mntpoint.clone();
    let onceflag_ctrlc = once_flag.clone();
    ctrlc::set_handler(move || {
        println!("Received Ctrl+C, unmounting...");
        cleanup(&mntpoint_ctrlc, &onceflag_ctrlc);
        exit(1);
    })?;

    let fs = MirrorFs::new(source_dir, DefaultFuseHandler::new());

    println!("Mounting mirror filesystem...");
    println!("Mount point: {:?}", &mntpoint);
    println!("Source directory: {:?}", fs.source_dir());

    // Mount the filesystem
    mount(fs, &mntpoint, &[], 1)?;

    // If we reach here, the filesystem has been unmounted normally
    cleanup(&mntpoint, &once_flag);

    Ok(())
}