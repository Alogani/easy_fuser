use std::path::PathBuf;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use clap::Parser;
use ctrlc;

mod helpers;
mod filesystem;
use filesystem::FtpFs;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// FTP server address (e.g., ftp.example.com:21)
    #[arg(short, long)]
    server: Option<String>,

    /// FTP username (use 'anonymous' for anonymous access)
    #[arg(short, long, default_value = "anonymous")]
    username: String,

    /// FTP password (can be empty for anonymous access)
    #[arg(short, long, default_value = "")]
    password: String,

    /// Mount point for the FTP filesystem
    #[arg(short, long)]
    mount_point: Option<PathBuf>,

    /// Directory cache size
    #[arg(short, long, default_value = "1000")]
    cache_size: usize,

    /// Positional arguments: [SERVER] [MOUNT_POINT]
    #[arg(required = false)]
    args: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let (server, mount_point) = if !args.args.is_empty() {
        if args.args.len() != 2 {
            return Err(
                "Expected exactly two positional arguments: <SERVER> <MOUNT_POINT>".into(),
            );
        }
        if args.server.is_some() || args.mount_point.is_some() {
            return Err("Cannot mix positional and named arguments".into());
        }
        (args.args[0].clone(), PathBuf::from(&args.args[1]))
    } else {
        let server = args.server.ok_or("FTP server address is required")?;
        let mount_point = args.mount_point.ok_or("Mount point is required")?;
        (server, mount_point)
    };
    let cache_size = args.cache_size;

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

    let ftp_fs = FtpFs::new(&server, &args.username, &args.password, cache_size)?;

    println!("Mounting FTP filesystem...");
    println!("FTP server: {}", &server);
    println!("Mount point: {:?}", &mount_point);

    // Mount the filesystem
    easy_fuser::mount(ftp_fs, &mount_point, &[], 4)?;

    // If we reach here, the filesystem has been unmounted normally
    cleanup(&mount_point, &once_flag);

    Ok(())
}