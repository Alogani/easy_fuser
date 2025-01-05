#![doc = include_str!("../README.md")]

mod crypto;
mod dir_handler;
mod filesystem;
mod helpers;
mod inode_hash_mapper;
use filesystem::EncryptFs;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use clap::Parser;
use ctrlc;

use std::io;
use std::path::PathBuf;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the directory to encrypt and mount
    #[arg(short, long)]
    source_dir: Option<PathBuf>,

    /// Mount point for the encrypted filesystem
    #[arg(short, long)]
    mount_point: Option<PathBuf>,

    /// Generate a new encryption key, key is returned as a base64-encoded string
    #[arg(short, long)]
    generate_key: bool,

    /// Positional arguments: [SOURCE_DIR] [MOUNT_POINT]
    #[arg(required = false)]
    args: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let (source_dir, mount_point) = if !args.args.is_empty() {
        if args.args.len() != 2 {
            return Err(
                "Expected exactly two positional arguments: <SOURCE_DIR> <MOUNT_POINT>".into(),
            );
        }
        if args.source_dir.is_some() || args.mount_point.is_some() {
            return Err("Cannot mix positional and named arguments".into());
        }
        (PathBuf::from(&args.args[0]), PathBuf::from(&args.args[1]))
    } else {
        let source_dir = args.source_dir.ok_or("Source directory path is required")?;
        let mount_point = args.mount_point.ok_or("Mount point is required")?;
        (source_dir, mount_point)
    };

    // Handle key generation or input
    let encryption_key = if args.generate_key {
        let key = helpers::generate_key();
        println!("Generated encryption key (base64 encoded):");
        println!("{}", BASE64.encode(key));
        key
    } else {
        // Read encryption key from stdin
        println!("Enter the encryption key (32 bytes, base64 encoded):");
        let mut key = String::new();
        io::stdin().read_line(&mut key)?;
        let key = key.trim();

        // Decode the base64 key
        let encryption_key = BASE64.decode(key)?;
        if encryption_key.len() != 32 {
            return Err("Encryption key must be 32 bytes long".into());
        }
        encryption_key.try_into().unwrap()
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

    let encrypt_fs = EncryptFs::new(source_dir.clone(), encryption_key);

    println!("Mounting encrypted filesystem...");
    println!("Source directory: {:?}", &source_dir);
    println!("Mount point: {:?}", &mount_point);

    // Mount the filesystem
    easy_fuser::mount(encrypt_fs, &mount_point, &[], 1)?;

    // If we reach here, the filesystem has been unmounted normally
    cleanup(&mount_point, &once_flag);

    Ok(())
}
