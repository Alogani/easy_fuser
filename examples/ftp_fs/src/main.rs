//! FtpFs: A read-only FUSE filesystem for FTP servers
//!
//! This program mounts an FTP server as a read-only filesystem using FUSE.
//! It allows browsing and reading the contents of the FTP server as if it were
//! a regular directory structure on the local machine.
//!
//! Usage:
//!     ftp_fs <SERVER> <PORT> <MOUNT_POINT>
//!     ftp_fs --server <SERVER> --port <PORT> --mount-point <MOUNT_POINT>
//!
//! Additional options:
//!     --username <USERNAME>    FTP username (default: "anonymous")
//!     --password <PASSWORD>    FTP password (default: "")
//!     --dir-detection <METHOD> Method to detect directories (default: "mlsd")
//!
//! IMPORTANT: This implementation is not recommended for production use.
//! It is highly unreliable, slow, and unoptimized. It may lead to unexpected
//! behavior, data inconsistencies, or performance issues. Use at your own risk
//! and only for educational or experimental purposes.

use std::path::PathBuf;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use clap::{Parser, ValueEnum};
use ctrlc;

mod filesystem;
mod helpers;
use filesystem::FtpFs;

#[derive(ValueEnum, Debug, PartialEq, Clone)]
pub enum DirectoryDetectionMethod {
    CwdCdup,
    List,
    Mlsd,
    FileSize,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// FTP server address (e.g., ftp.example.com)
    #[arg(short, long)]
    server: Option<String>,

    /// FTP server port (e.g., 21)
    #[arg(short, long)]
    port: Option<u32>,

    /// FTP username (use 'anonymous' for anonymous access)
    #[arg(long, default_value = "anonymous")]
    username: String,

    /// FTP password (can be empty for anonymous access)
    #[arg(long, default_value = "")]
    password: String,

    /// Mount point for the FTP filesystem
    #[arg(short, long)]
    mount_point: Option<PathBuf>,

    /// Method to detect directories, unreliable and dependant on the server
    #[arg(long, default_value = "mlsd")]
    dir_detection: DirectoryDetectionMethod,

    /// Positional arguments: [SERVER] [MOUNT_POINT]
    #[arg(required = false)]
    args: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "logging")]
    std::env::set_var("RUST_BACKTRACE", "full");
    #[cfg(feature = "logging")]
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let args = Args::parse();
    let (server, port, mount_point) = if !args.args.is_empty() {
        if args.args.len() != 3 {
            return Err(
                "Expected exactly three positional arguments: <SERVER> <PORT> <MOUNT_POINT>".into(),
            );
        }
        if args.server.is_some() || args.mount_point.is_some() {
            return Err("Cannot mix positional and named arguments".into());
        }
        (
            args.args[0].clone(),
            args.args[1].parse::<u32>()?,
            PathBuf::from(&args.args[2]),
        )
    } else {
        let server = args.server.ok_or("FTP server address is required")?;
        let port = args.port.ok_or("FTP server port is required")?;
        let mount_point = args.mount_point.ok_or("Mount point is required")?;
        (server, port, mount_point)
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

    let ftp_fs = FtpFs::new(
        &server,
        &args.username,
        port,
        &args.password,
        args.dir_detection,
    )?;

    println!("Mounting FTP filesystem...");
    println!("FTP server: {}", &server);
    println!("Mount point: {:?}", &mount_point);

    // Mount the filesystem
    easy_fuser::mount(ftp_fs, &mount_point, &[], 4)?;

    // If we reach here, the filesystem has been unmounted normally
    cleanup(&mount_point, &once_flag);

    Ok(())
}
