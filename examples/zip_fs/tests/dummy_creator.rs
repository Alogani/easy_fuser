#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! zip = "2.2.2"
//! ```

use std::{fs::File, io::Write};
use zip::write::FileOptions;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} ZIP_PATH", args[0]);
        std::process::exit(1);
    }
    let zip_path = &args[1];

    let file = File::create(zip_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::<()>::default()//+
        .compression_method(zip::CompressionMethod::Stored)//+
        .unix_permissions(0o755);

    zip.start_file("file1.txt", options).unwrap();
    zip.write_all(b"Content of file1\n").unwrap();

    zip.start_file("file2.txt", options).unwrap();
    zip.write_all(b"Content of file2\n").unwrap();

    zip.add_directory("subdir", options).unwrap();

    zip.start_file("subdir/file3.txt", options).unwrap();
    zip.write_all(b"Content of file3\n").unwrap();

    zip.finish().unwrap();
}