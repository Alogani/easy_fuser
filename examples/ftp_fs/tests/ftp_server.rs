#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! libunftp = "0.20"
//! unftp-sbe-fs = "0.2"
//! async-trait = "0.1.68"
//! tokio = { version = "1.42", features = ["full"] }
//! ```

use std::path::PathBuf;
use std::{env, error::Error};
use tokio::signal;
use tokio::task::JoinHandle;
use tokio::runtime::Runtime;

use unftp_sbe_fs::ServerExt;


fn run_ftp_server(serve_dir: PathBuf, port: u16) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    tokio::spawn(async move {
        let server = libunftp::Server::with_fs(serve_dir)
            .greeting("Welcome to my FTP server")
            .passive_ports(50000..65535)
            .build()
            .unwrap();

        server.listen(format!("127.0.0.1:{}", port)).await?;

        Ok(())
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <directory> <port>", args[0]);
        std::process::exit(1);
    }

    let serve_dir = PathBuf::from(&args[1]);
    let port: u16 = args[2].parse()?;

    let rt  = Runtime::new().unwrap();
    let handle = rt.handle();
    let res = handle.block_on(async {
        eprintln!("Starting FTP server on 127.0.0.1:{}", port);
        let server_handle = run_ftp_server(serve_dir, port);
        
        tokio::select! {
            result = server_handle => {
                match result {
                    Ok(inner_result) => {
                        match inner_result {
                            Ok(_) => Ok(()),
                            Err(e) => Err(e.to_string()),
                        }
                    },
                    Err(e) => Err(e.to_string()),
                }
            }
            _ = signal::ctrl_c() => {
                eprintln!("Received Ctrl+C, shutting down...");
                Ok(())
            }
        }
    });
    Ok(res?)
}