//! A read-only FUSE filesystem for FTP servers using easy_fuser

mod filesystem;
mod helpers;

use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use clap::Parser;
use ctrlc;

use filesystem::FtpFs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ftp_fs = FtpFs::new("ftp.example.com:21", "username", "password", 1000)?;
    
    easy_fuser::mount(ftp_fs, Path::new("/mnt/ftp"), &[], 4)?;
    
    Ok(())
}