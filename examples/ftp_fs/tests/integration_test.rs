use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
use tempfile::TempDir;
use std::io::Read;

#[test]
fn test_ftp_fs_mount_and_read() {
    // Create a temporary directory for our test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mount_point = temp_dir.path().join("mnt");

    // Create mount point directory
    fs::create_dir(&mount_point).expect("Failed to create mount point");

    // Run the ftp_fs command
    let mut child = Command::new(env!("CARGO_BIN_EXE_ftp_fs"))
        .arg("ftp.de.debian.org/debian/")
        .arg(&mount_point)
        .arg("--username")
        .arg("anonymous")
        .arg("--password")
        .arg("")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to execute ftp_fs");

    // Wait for the filesystem to mount
    sleep(Duration::from_secs(5));

    // Check if the mounted filesystem contains the expected file
    assert!(mount_point.join("doc/constitution.txt").exists());

    // Read and verify file contents
    let mut file = File::open(mount_point.join("doc/constitution.txt")).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    assert!(contents.contains("Constitution for the Debian Project"));

    // Read the doc directory and check the number of files
    let entries = fs::read_dir(mount_point.join("doc")).unwrap();
    let file_count = entries.count();
    assert!(file_count > 1, "The doc directory should contain more than one file");

    // Unmount the filesystem
    Command::new("fusermount")
        .arg("-u")
        .arg(&mount_point)
        .status()
        .expect("Failed to unmount filesystem");

    // Terminate the ftp_fs process
    child.kill().expect("Failed to kill ftp_fs process");

    // Clean up
    temp_dir.close().expect("Failed to clean up temp directory");
}