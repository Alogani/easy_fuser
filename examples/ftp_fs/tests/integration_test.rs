use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
use tempfile::TempDir;

mod helpers;
use helpers::ftp_server::spawn_ftp_server;

pub fn populate_test_directory(base_dir: &Path) -> std::io::Result<()> {
    // Create the base directory if it doesn't exist
    fs::create_dir_all(base_dir)?;

    // Create a folder inside the base directory
    let folder_path = base_dir.join("test_folder");
    fs::create_dir(&folder_path)?;

    // Create a file inside the folder
    let file_path = folder_path.join("hello.txt");
    let mut file = fs::File::create(file_path)?;

    // Write "Hello World!\n" to the file
    file.write_all(b"Hello World!\n")?;

    Ok(())
}

#[test]
fn test_ftp_fs_mount_and_read() {
    // Create a temporary directory for our test
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mount_point = TempDir::new().expect("Failed to create mountpoint directory");
    let mount_path = mount_point.path();

    populate_test_directory(temp_dir.path()).unwrap();
    let _ = spawn_ftp_server(temp_dir.path(), 9991);

    // Wait for the server to start
    sleep(Duration::from_millis(50));

    eprintln!("mounting filesystem...");
    // Run the ftp_fs command
    let mut child = Command::new(env!("CARGO_BIN_EXE_ftp_fs"))
        .arg("127.0.0.1")
        .arg("9991")
        .arg(mount_path)
        .arg("--username")
        .arg("anonymous")
        .arg("--password")
        .arg("")
        .arg("--dir-detection")
        .arg("list")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to execute ftp_fs");

    // Wait for the filesystem to mount
    sleep(Duration::from_millis(50));
    eprintln!("filesystem mounted, begin testing...");

    // Check if the mounted filesystem contains the expected folder and file
    assert!(mount_path.join("test_folder").exists());
    assert!(mount_path.join("test_folder/hello.txt").exists());

    // Read and verify file contents
    let mut file = File::open(mount_path.join("test_folder/hello.txt")).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, "Hello World!\n");

    // Read the base directory and check the number of entries
    let entries = fs::read_dir(&mount_path).unwrap();
    let entry_count = entries.count();
    assert_eq!(
        entry_count, 1,
        "The base directory should contain exactly one entry (test_folder)"
    );

    // Read the test_folder directory and check the number of files
    let entries = fs::read_dir(mount_path.join("test_folder")).unwrap();
    let file_count = entries.count();
    assert_eq!(
        file_count, 1,
        "The test_folder should contain exactly one file (hello.txt)"
    );

    // Unmount the filesystem
    Command::new("fusermount")
        .arg("-u")
        .arg(&mount_path)
        .status()
        .expect("Failed to unmount filesystem");

    // Terminate the ftp_fs process
    child.kill().expect("Failed to kill ftp_fs process");

    // Clean up
    temp_dir.close().expect("Failed to clean up temp directory");
}
