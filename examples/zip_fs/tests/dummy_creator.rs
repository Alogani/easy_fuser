#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! zip = "2.2.2"
//! ```

use std::{fs::File, io::Write, path::Path};
use zip::write::FileOptions;

#[allow(dead_code)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} ZIP_PATH", args[0]);
        std::process::exit(1);
    }
    let zip_path = &args[1];
    create_dummy_zip(zip_path).unwrap();
}

pub fn create_dummy_zip<P: AsRef<Path>>(zip_path: P) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(zip_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::<()>::default() //+
        .compression_method(zip::CompressionMethod::Stored) //+
        .unix_permissions(0o755);

    zip.start_file("file1.txt", options)?;
    zip.write_all(b"Content of file1\n")?;

    zip.start_file("file2.txt", options)?;
    zip.write_all(b"Content of file2\n")?;

    zip.add_directory("subdir", options)?;

    zip.start_file("subdir/file3.txt", options)?;
    zip.write_all(b"Content of file3\n")?;

    zip.finish()?;
    Ok(())
}
