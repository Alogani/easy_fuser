#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! libunftp = "0.20"
//! unftp-sbe-fs = "0.2"
//! async-trait = "0.1.68"
//! tokio = { version = "1.42", features = ["full"] }
//! ```

use std::path::{Path, PathBuf};
use std::{env, error::Error};
use tokio::runtime::Runtime;
use tokio::signal;

use unftp_sbe_fs::ServerExt;

pub fn spawn_ftp_server(serve_dir: &Path, port: u16) -> std::thread::JoinHandle<()> {
    let rt = Runtime::new().unwrap();
    let serve_dir = serve_dir.to_owned();
    std::thread::spawn(move || {
        rt.block_on(async {
            if let Err(e) = run_ftp_server(&serve_dir, port).await {
                eprintln!("FTP server error: {}", e);
            }
        });
    })
}

pub async fn run_ftp_server(
    serve_dir: &Path,
    port: u16,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    eprintln!("Starting FTP server on 127.0.0.1:{}", port);
    let serve_dir = PathBuf::from(serve_dir);

    let server = libunftp::Server::with_fs(serve_dir)
        .greeting("Welcome to my FTP server")
        .passive_ports(50000..65535)
        .build()
        .unwrap();

    let server_handle = tokio::spawn(server.listen(format!("127.0.0.1:{}", port)));

    tokio::select! {
        result = server_handle => {
            match result? {
                Ok(_) => (),
                Err(e) => Err(e.to_string())?
            }
        }
        _ = signal::ctrl_c() => {
            println!("Received Ctrl+C, shutting down FTP server...");
        }
    }

    Ok(())
}

#[allow(dead_code)]
fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <directory> <port>", args[0]);
        std::process::exit(1);
    }

    let serve_dir = PathBuf::from(&args[1]);
    let port: u16 = args[2].parse()?;

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();
    let res = handle.block_on(async { run_ftp_server(&serve_dir, port).await });
    Ok(res?)
}
