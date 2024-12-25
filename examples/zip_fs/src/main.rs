//! ZipFs: A read-only FUSE filesystem for ZIP archives
//! 
//! This program mounts a ZIP archive as a read-only filesystem using FUSE.
//! It allows browsing and reading the contents of the ZIP file as if it were
//! a regular directory structure.
//!
//! Usage:
//!     zip_fs <ZIP_FILE> <MOUNT_POINT>
//!     zip_fs --zip-file <ZIP_FILE> --mount-point <MOUNT_POINT>

mod filesystem;
mod helpers;

use std::path::PathBuf;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use clap::Parser;
use ctrlc;

use filesystem::ZipFs;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the ZIP file to mount
    #[arg(short, long)]
    zip_file: Option<PathBuf>,

    /// Mount point for the ZIP filesystem
    #[arg(short, long)]
    mount_point: Option<PathBuf>,

    /// Positional arguments: [ZIP_FILE] [MOUNT_POINT]
    #[arg(required = false)]
    args: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_BACKTRACE", "full");
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Trace)
        .try_init();


    let args = Args::parse();
    let (zip_file, mount_point) = if !args.args.is_empty() {
        if args.args.len() != 2 {
            return Err("Expected exactly two positional arguments: <ZIP_FILE> <MOUNT_POINT>".into());
        }
        if args.zip_file.is_some() || args.mount_point.is_some() {
            return Err("Cannot mix positional and named arguments".into());
        }
        (args.args[0].clone(), args.args[1].clone())
    } else {
        let zip_file = args.zip_file.ok_or("ZIP file path is required")?;
        let mount_point = args.mount_point.ok_or("Mount point is required")?;
        (zip_file, mount_point)
    };

    // Ensure the mount point exists
    std::fs::create_dir_all(&mount_point)?;

    // Set up the cleanup function
    let once_flag = Arc::new(AtomicBool::new(false));
    let cleanup = |mount_point: &PathBuf, once_flag: &Arc<AtomicBool>| {
        if once_flag.clone().swap(true, Ordering::SeqCst) {
            return;
        }
        println!("Unmounting filesystem...");
        let _ = std::process::Command::new("fusermount")
            .arg("-u")
            .arg(mount_point)
            .status();
    };

    // Set up Ctrl+C handler
    let mount_point_ctrlc = mount_point.clone();
    let onceflag_ctrlc = once_flag.clone();
    ctrlc::set_handler(move || {
        println!("Received Ctrl+C, unmounting...");
        cleanup(&mount_point_ctrlc, &onceflag_ctrlc);
        exit(1);
    })?;

    let zip_fs = ZipFs::new(&zip_file)?;

    println!("Mounting ZIP filesystem...");
    println!("ZIP file: {:?}", &zip_file);
    println!("Mount point: {:?}", &mount_point);

    // Mount the filesystem
    easy_fuser::mount(zip_fs, &mount_point, &[], 1)?;

    // If we reach here, the filesystem has been unmounted normally
    cleanup(&mount_point, &once_flag);

    Ok(())
}