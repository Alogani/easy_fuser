use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
use tempfile::TempDir;

mod dummy_creator;
use dummy_creator::create_dummy_zip;


#[test]
fn test_zip_fs_mount_and_read() {
    // Create a temporary directory for our test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let zip_path = temp_dir.path().join("test.zip");
    let mount_point = temp_dir.path().join("mnt");

    // Create a test ZIP file
    create_dummy_zip(&zip_path).unwrap();
    
    // Verify that the ZIP file was created
    assert!(zip_path.exists());

    // Verify that the ZIP file contains the expected files
    let mut zip_archive = zip::ZipArchive::new(File::open(&zip_path).expect("Failed to open zip file")).unwrap();
    assert_eq!(zip_archive.file_names().count(), 4);
    assert_eq!(zip_archive.by_name("file1.txt").is_ok(), true);
    assert_eq!(zip_archive.by_name("file2.txt").is_ok(), true);
    assert_eq!(zip_archive.by_name("subdir/file3.txt").is_ok(), true);

    // Create mount point directory
    fs::create_dir(&mount_point).expect("Failed to create mount point");

    // Run the zip_fs command
    let mut child = Command::new(env!("CARGO_BIN_EXE_zip_fs"))
        .arg(&zip_path)
        .arg(&mount_point)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to execute zip_fs");

    // Wait for the filesystem to mount
    sleep(Duration::from_secs(2));

    // Check if the mounted filesystem contains the expected files
    assert!(mount_point.join("file1.txt").exists());
    assert!(mount_point.join("file2.txt").exists());
    assert!(mount_point.join("subdir").is_dir());
    assert!(mount_point.join("subdir/file3.txt").exists());

    // Read and verify file contents
    assert_eq!(fs::read_to_string(mount_point.join("file1.txt")).unwrap(), "Content of file1\n");
    assert_eq!(fs::read_to_string(mount_point.join("file2.txt")).unwrap(), "Content of file2\n");
    assert_eq!(fs::read_to_string(mount_point.join("subdir/file3.txt")).unwrap(), "Content of file3\n");

    // Unmount the filesystem
    Command::new("fusermount")
        .arg("-u")
        .arg(&mount_point)
        .status()
        .expect("Failed to unmount filesystem");

    // Terminate the zip_fs process
    child.kill().expect("Failed to kill zip_fs process");

    // Clean up
    temp_dir.close().expect("Failed to clean up temp directory");
}