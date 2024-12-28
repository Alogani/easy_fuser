use std::fs::File;
use std::io::Read;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_debian_ftp_mount_and_read() {
    // Create a temporary directory for our mount point
    let mount_point = TempDir::new().expect("Failed to create mountpoint directory");
    let mount_path = mount_point.path();

    eprintln!("Mounting Debian FTP filesystem...");
    // Run the ftp_fs command
    let mut child = Command::new(env!("CARGO_BIN_EXE_ftp_fs"))
        .arg("-s")
        .arg("ftp.de.debian.org")
        .arg("-p")
        .arg("21")
        .arg("-m")
        .arg(mount_path)
        .arg("--dir-detection")
        .arg("mlsd")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to execute ftp_fs");

    // Wait for the filesystem to mount
    sleep(Duration::from_secs(5));
    eprintln!("Filesystem mounted, begin testing...");

    // Check if the mounted filesystem contains the expected file
    let file_path = mount_path.join("debian/doc/constitution.txt");
    assert!(
        file_path.exists(),
        "The constitution.txt file does not exist"
    );

    // Read file contents as bytes
    let mut file = File::open(&file_path).expect("Failed to open constitution.txt");
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)
        .expect("Failed to read constitution.txt");

    // Check if the file begins with the expected content
    let expected_content = b"Constitution for the Debian Project";
    assert!(
        contents
            .windows(expected_content.len())
            .any(|window| window == expected_content),
        "The file does not contain the expected content"
    );

    eprintln!("File content verified successfully");

    // Unmount the filesystem
    Command::new("fusermount")
        .arg("-u")
        .arg(&mount_path)
        .status()
        .expect("Failed to unmount filesystem");

    // Terminate the ftp_fs process
    child.kill().expect("Failed to kill ftp_fs process");

    eprintln!("Test completed successfully");
}
